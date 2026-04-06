//! 本地音频播放与录制辅助能力。

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use bytes::Bytes;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

use crate::error::{Error, Result};
use crate::files::UploadSource;

const DEFAULT_SAMPLE_RATE: u32 = 24_000;
const DEFAULT_CHANNELS: u32 = 1;

/// 表示可用于本地播放的音频输入。
#[derive(Debug, Clone)]
pub enum AudioPlaybackInput {
    /// 直接播放本地文件。
    Path(PathBuf),
    /// 通过 stdin 管道播放内存中的音频字节。
    Bytes(Bytes),
    /// 通过上传源中的字节进行播放。
    UploadSource(UploadSource),
}

impl AudioPlaybackInput {
    /// 从路径创建播放输入。
    pub fn path(path: impl Into<PathBuf>) -> Self {
        Self::Path(path.into())
    }

    /// 从字节创建播放输入。
    pub fn bytes(bytes: impl Into<Bytes>) -> Self {
        Self::Bytes(bytes.into())
    }

    /// 从上传源创建播放输入。
    pub fn upload(source: UploadSource) -> Self {
        Self::UploadSource(source)
    }
}

impl From<PathBuf> for AudioPlaybackInput {
    fn from(value: PathBuf) -> Self {
        Self::Path(value)
    }
}

impl From<&Path> for AudioPlaybackInput {
    fn from(value: &Path) -> Self {
        Self::Path(value.to_path_buf())
    }
}

impl From<Vec<u8>> for AudioPlaybackInput {
    fn from(value: Vec<u8>) -> Self {
        Self::Bytes(Bytes::from(value))
    }
}

impl From<Bytes> for AudioPlaybackInput {
    fn from(value: Bytes) -> Self {
        Self::Bytes(value)
    }
}

impl From<UploadSource> for AudioPlaybackInput {
    fn from(value: UploadSource) -> Self {
        Self::UploadSource(value)
    }
}

/// 表示录音辅助的可调参数。
#[derive(Debug, Clone)]
pub struct RecordAudioOptions {
    /// 指定采集设备编号或名称，默认使用 `0`。
    pub device: Option<String>,
    /// 录音超时时长，超时后会主动停止采集。
    pub timeout: Option<Duration>,
    /// 录音采样率，默认 `24000`。
    pub sample_rate: u32,
    /// 录音声道数，默认 `1`。
    pub channels: u32,
    /// 覆盖平台默认输入 provider。
    pub provider: Option<String>,
    /// 覆盖输出文件名，默认 `audio.wav`。
    pub filename: String,
    /// 覆盖录音程序名，默认 `ffmpeg`。
    pub program: String,
}

impl Default for RecordAudioOptions {
    fn default() -> Self {
        Self {
            device: None,
            timeout: None,
            sample_rate: DEFAULT_SAMPLE_RATE,
            channels: DEFAULT_CHANNELS,
            provider: None,
            filename: "audio.wav".into(),
            program: "ffmpeg".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommandSpec {
    program: String,
    args: Vec<String>,
    stdin: Option<Bytes>,
}

/// 使用系统中的 `ffplay` 播放音频。
///
/// 当输入为字节或上传源时，会通过 `stdin` 管道向播放器传输数据。
///
/// # Errors
///
/// 当本地播放器不存在、启动失败或退出码非零时返回错误。
pub async fn play_audio(input: impl Into<AudioPlaybackInput>) -> Result<()> {
    let spec = build_play_audio_command(input.into(), "ffplay");
    run_play_command(spec).await
}

/// 使用系统中的 `ffmpeg` 录制一段音频并返回统一上传源。
///
/// # Errors
///
/// 当当前平台缺少默认采集 provider、命令执行失败或录制超时时返回错误。
pub async fn record_audio(options: RecordAudioOptions) -> Result<UploadSource> {
    let spec = build_record_audio_command(&options, std::env::consts::OS)?;
    let bytes = run_record_command(spec, options.timeout).await?;
    Ok(UploadSource::from_bytes(bytes, options.filename).with_mime_type("audio/wav"))
}

fn build_play_audio_command(input: AudioPlaybackInput, program: &str) -> CommandSpec {
    match input {
        AudioPlaybackInput::Path(path) => CommandSpec {
            program: program.into(),
            args: vec![
                "-autoexit".into(),
                "-nodisp".into(),
                "-i".into(),
                path.to_string_lossy().into_owned(),
            ],
            stdin: None,
        },
        AudioPlaybackInput::Bytes(bytes) => CommandSpec {
            program: program.into(),
            args: vec![
                "-autoexit".into(),
                "-nodisp".into(),
                "-i".into(),
                "pipe:0".into(),
            ],
            stdin: Some(bytes),
        },
        AudioPlaybackInput::UploadSource(source) => CommandSpec {
            program: program.into(),
            args: vec![
                "-autoexit".into(),
                "-nodisp".into(),
                "-i".into(),
                "pipe:0".into(),
            ],
            stdin: Some(source.bytes().clone()),
        },
    }
}

fn build_record_audio_command(options: &RecordAudioOptions, platform: &str) -> Result<CommandSpec> {
    let provider = if let Some(provider) = &options.provider {
        provider.clone()
    } else {
        default_recording_provider(platform)
            .ok_or_else(|| {
                Error::InvalidConfig(format!("当前平台 `{platform}` 不支持默认录音 provider"))
            })?
            .into()
    };
    let device = options.device.as_deref().unwrap_or("0");

    Ok(CommandSpec {
        program: options.program.clone(),
        args: vec![
            "-f".into(),
            provider,
            "-i".into(),
            format!(":{device}"),
            "-ar".into(),
            options.sample_rate.to_string(),
            "-ac".into(),
            options.channels.to_string(),
            "-f".into(),
            "wav".into(),
            "pipe:1".into(),
        ],
        stdin: None,
    })
}

async fn run_play_command(spec: CommandSpec) -> Result<()> {
    let mut command = Command::new(&spec.program);
    command.args(&spec.args);
    command.stdout(Stdio::null()).stderr(Stdio::null());
    if spec.stdin.is_some() {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }

    let mut child = command
        .spawn()
        .map_err(|error| Error::InvalidConfig(format!("启动 `{}` 失败: {error}", spec.program)))?;

    if let Some(bytes) = spec.stdin {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::InvalidConfig(format!("`{}` 未暴露 stdin 管道", spec.program)))?;
        stdin.write_all(&bytes).await.map_err(|error| {
            Error::InvalidConfig(format!("向 `{}` 写入音频失败: {error}", spec.program))
        })?;
        stdin.shutdown().await.map_err(|error| {
            Error::InvalidConfig(format!("关闭 `{}` stdin 失败: {error}", spec.program))
        })?;
    }

    let status = child.wait().await.map_err(|error| {
        Error::InvalidConfig(format!("等待 `{}` 退出失败: {error}", spec.program))
    })?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::InvalidConfig(format!(
            "`{}` 退出失败，状态码: {status}",
            spec.program
        )))
    }
}

async fn run_record_command(spec: CommandSpec, timeout: Option<Duration>) -> Result<Bytes> {
    let mut command = Command::new(&spec.program);
    command.args(&spec.args);
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::null());

    let mut child = command
        .spawn()
        .map_err(|error| Error::InvalidConfig(format!("启动 `{}` 失败: {error}", spec.program)))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| Error::InvalidConfig(format!("`{}` 未暴露 stdout 管道", spec.program)))?;
    let read_stdout = tokio::spawn(async move {
        let mut buffer = Vec::new();
        stdout.read_to_end(&mut buffer).await.map(|_| buffer)
    });

    let status = if let Some(timeout) = timeout {
        tokio::select! {
            status = child.wait() => {
                status.map_err(|error| Error::InvalidConfig(format!("等待 `{}` 退出失败: {error}", spec.program)))?
            }
            _ = tokio::time::sleep(timeout) => {
                let _ = child.start_kill();
                let _ = child.wait().await;
                return Err(Error::Timeout);
            }
        }
    } else {
        child.wait().await.map_err(|error| {
            Error::InvalidConfig(format!("等待 `{}` 退出失败: {error}", spec.program))
        })?
    };

    let bytes = read_stdout
        .await
        .map_err(|error| {
            Error::InvalidConfig(format!("读取 `{}` 输出失败: {error}", spec.program))
        })?
        .map_err(|error| {
            Error::InvalidConfig(format!("读取 `{}` 输出失败: {error}", spec.program))
        })?;

    if status.success() {
        Ok(Bytes::from(bytes))
    } else {
        Err(Error::InvalidConfig(format!(
            "`{}` 退出失败，状态码: {status}",
            spec.program
        )))
    }
}

fn default_recording_provider(platform: &str) -> Option<&'static str> {
    match platform {
        "windows" => Some("dshow"),
        "macos" => Some("avfoundation"),
        "linux" | "android" | "freebsd" | "haiku" | "netbsd" | "openbsd" => Some("alsa"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AudioPlaybackInput, RecordAudioOptions, build_play_audio_command,
        build_record_audio_command, default_recording_provider,
    };
    use bytes::Bytes;

    #[test]
    fn test_should_build_play_command_for_path_input() {
        let spec = build_play_audio_command(AudioPlaybackInput::path("/tmp/sample.wav"), "ffplay");
        assert_eq!(
            spec.args,
            vec!["-autoexit", "-nodisp", "-i", "/tmp/sample.wav"]
        );
        assert!(spec.stdin.is_none());
    }

    #[test]
    fn test_should_build_play_command_for_bytes_input() {
        let spec = build_play_audio_command(
            AudioPlaybackInput::bytes(Bytes::from_static(b"wav")),
            "ffplay",
        );
        assert_eq!(spec.args, vec!["-autoexit", "-nodisp", "-i", "pipe:0"]);
        assert_eq!(spec.stdin, Some(Bytes::from_static(b"wav")));
    }

    #[test]
    fn test_should_build_record_command_with_platform_defaults() {
        let spec = build_record_audio_command(&RecordAudioOptions::default(), "linux").unwrap();
        assert_eq!(
            spec.args,
            vec![
                "-f", "alsa", "-i", ":0", "-ar", "24000", "-ac", "1", "-f", "wav", "pipe:1"
            ]
        );
    }

    #[test]
    fn test_should_fail_when_platform_has_no_default_provider() {
        let error =
            build_record_audio_command(&RecordAudioOptions::default(), "dragonfly").unwrap_err();
        assert!(matches!(error, crate::Error::InvalidConfig(_)));
    }

    #[test]
    fn test_should_map_platform_provider() {
        assert_eq!(default_recording_provider("macos"), Some("avfoundation"));
        assert_eq!(default_recording_provider("windows"), Some("dshow"));
        assert_eq!(default_recording_provider("linux"), Some("alsa"));
    }
}
