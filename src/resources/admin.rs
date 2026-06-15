//! Admin resource namespace implementations.

use http::Method;
use serde_json::Value;

use crate::pagination::Page;

use super::{
    AdminOrganizationAdminApiKeysResource, AdminOrganizationAuditLogsResource,
    AdminOrganizationCertificatesResource, AdminOrganizationDataRetentionResource,
    AdminOrganizationGroupRolesResource, AdminOrganizationGroupUsersResource,
    AdminOrganizationGroupsResource, AdminOrganizationInvitesResource,
    AdminOrganizationProjectsResource, AdminOrganizationResource, AdminOrganizationRolesResource,
    AdminOrganizationSpendAlertsResource, AdminOrganizationUsageResource,
    AdminOrganizationUserRolesResource, AdminOrganizationUsersResource,
    AdminProjectApiKeysResource, AdminProjectCertificatesResource,
    AdminProjectDataRetentionResource, AdminProjectGroupRolesResource, AdminProjectGroupsResource,
    AdminProjectHostedToolPermissionsResource, AdminProjectModelPermissionsResource,
    AdminProjectRateLimitsResource, AdminProjectRolesResource, AdminProjectServiceAccountsResource,
    AdminProjectSpendAlertsResource, AdminProjectUserRolesResource, AdminProjectUsersResource,
    AdminResource, JsonRequestBuilder, ListRequestBuilder, encode_path_segment,
};

fn enc(value: impl Into<String>) -> String {
    encode_path_segment(value.into())
}

impl AdminResource {
    /// Returns the organization admin namespace.
    pub fn organization(&self) -> AdminOrganizationResource {
        AdminOrganizationResource::new(self.client.clone())
    }
}

impl AdminOrganizationResource {
    /// Returns audit log resources.
    pub fn audit_logs(&self) -> AdminOrganizationAuditLogsResource {
        AdminOrganizationAuditLogsResource::new(self.client.clone())
    }

    /// Returns organization admin API key resources.
    pub fn admin_api_keys(&self) -> AdminOrganizationAdminApiKeysResource {
        AdminOrganizationAdminApiKeysResource::new(self.client.clone())
    }

    /// Returns organization usage resources.
    pub fn usage(&self) -> AdminOrganizationUsageResource {
        AdminOrganizationUsageResource::new(self.client.clone())
    }

    /// Returns organization invite resources.
    pub fn invites(&self) -> AdminOrganizationInvitesResource {
        AdminOrganizationInvitesResource::new(self.client.clone())
    }

    /// Returns organization user resources.
    pub fn users(&self) -> AdminOrganizationUsersResource {
        AdminOrganizationUsersResource::new(self.client.clone())
    }

    /// Returns organization group resources.
    pub fn groups(&self) -> AdminOrganizationGroupsResource {
        AdminOrganizationGroupsResource::new(self.client.clone())
    }

    /// Returns organization role resources.
    pub fn roles(&self) -> AdminOrganizationRolesResource {
        AdminOrganizationRolesResource::new(self.client.clone())
    }

    /// Returns organization data retention resources.
    pub fn data_retention(&self) -> AdminOrganizationDataRetentionResource {
        AdminOrganizationDataRetentionResource::new(self.client.clone())
    }

    /// Returns organization spend alert resources.
    pub fn spend_alerts(&self) -> AdminOrganizationSpendAlertsResource {
        AdminOrganizationSpendAlertsResource::new(self.client.clone())
    }

    /// Returns organization certificate resources.
    pub fn certificates(&self) -> AdminOrganizationCertificatesResource {
        AdminOrganizationCertificatesResource::new(self.client.clone())
    }

    /// Returns organization project resources.
    pub fn projects(&self) -> AdminOrganizationProjectsResource {
        AdminOrganizationProjectsResource::new(self.client.clone())
    }
}

impl AdminOrganizationAdminApiKeysResource {
    /// Creates an organization admin API key.
    pub fn create(&self) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.admin_api_keys.create",
            Method::POST,
            "/organization/admin_api_keys",
        )
    }

    /// Retrieves an organization admin API key.
    pub fn retrieve(&self, key_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.admin_api_keys.retrieve",
            Method::GET,
            format!("/organization/admin_api_keys/{}", enc(key_id)),
        )
    }

    /// Lists organization admin API keys.
    pub fn list(&self) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.admin_api_keys.list",
            "/organization/admin_api_keys",
        )
    }

    /// Deletes an organization admin API key.
    pub fn delete(&self, key_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.admin_api_keys.delete",
            Method::DELETE,
            format!("/organization/admin_api_keys/{}", enc(key_id)),
        )
    }
}

impl AdminOrganizationAuditLogsResource {
    /// Lists organization audit logs.
    pub fn list(&self) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.audit_logs.list",
            "/organization/audit_logs",
        )
    }
}

impl AdminOrganizationUsageResource {
    /// Retrieves audio speech usage.
    pub fn audio_speeches(&self) -> JsonRequestBuilder<Value> {
        usage_get(
            self,
            "admin.organization.usage.audio_speeches",
            "/organization/usage/audio_speeches",
        )
    }

    /// Retrieves audio transcription usage.
    pub fn audio_transcriptions(&self) -> JsonRequestBuilder<Value> {
        usage_get(
            self,
            "admin.organization.usage.audio_transcriptions",
            "/organization/usage/audio_transcriptions",
        )
    }

    /// Retrieves code interpreter session usage.
    pub fn code_interpreter_sessions(&self) -> JsonRequestBuilder<Value> {
        usage_get(
            self,
            "admin.organization.usage.code_interpreter_sessions",
            "/organization/usage/code_interpreter_sessions",
        )
    }

    /// Retrieves completion usage.
    pub fn completions(&self) -> JsonRequestBuilder<Value> {
        usage_get(
            self,
            "admin.organization.usage.completions",
            "/organization/usage/completions",
        )
    }

    /// Retrieves cost usage.
    pub fn costs(&self) -> JsonRequestBuilder<Value> {
        usage_get(
            self,
            "admin.organization.usage.costs",
            "/organization/costs",
        )
    }

    /// Retrieves embedding usage.
    pub fn embeddings(&self) -> JsonRequestBuilder<Value> {
        usage_get(
            self,
            "admin.organization.usage.embeddings",
            "/organization/usage/embeddings",
        )
    }

    /// Retrieves file search call usage.
    pub fn file_search_calls(&self) -> JsonRequestBuilder<Value> {
        usage_get(
            self,
            "admin.organization.usage.file_search_calls",
            "/organization/usage/file_search_calls",
        )
    }

    /// Retrieves image usage.
    pub fn images(&self) -> JsonRequestBuilder<Value> {
        usage_get(
            self,
            "admin.organization.usage.images",
            "/organization/usage/images",
        )
    }

    /// Retrieves moderation usage.
    pub fn moderations(&self) -> JsonRequestBuilder<Value> {
        usage_get(
            self,
            "admin.organization.usage.moderations",
            "/organization/usage/moderations",
        )
    }

    /// Retrieves vector store usage.
    pub fn vector_stores(&self) -> JsonRequestBuilder<Value> {
        usage_get(
            self,
            "admin.organization.usage.vector_stores",
            "/organization/usage/vector_stores",
        )
    }

    /// Retrieves web search call usage.
    pub fn web_search_calls(&self) -> JsonRequestBuilder<Value> {
        usage_get(
            self,
            "admin.organization.usage.web_search_calls",
            "/organization/usage/web_search_calls",
        )
    }
}

fn usage_get(
    resource: &AdminOrganizationUsageResource,
    endpoint_id: &'static str,
    path: &'static str,
) -> JsonRequestBuilder<Value> {
    JsonRequestBuilder::new(resource.client.clone(), endpoint_id, Method::GET, path)
}

impl AdminOrganizationInvitesResource {
    /// Creates an organization invite.
    pub fn create(&self) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.invites.create",
            Method::POST,
            "/organization/invites",
        )
    }

    /// Retrieves an organization invite.
    pub fn retrieve(&self, invite_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.invites.retrieve",
            Method::GET,
            format!("/organization/invites/{}", enc(invite_id)),
        )
    }

    /// Lists organization invites.
    pub fn list(&self) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.invites.list",
            "/organization/invites",
        )
    }

    /// Deletes an organization invite.
    pub fn delete(&self, invite_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.invites.delete",
            Method::DELETE,
            format!("/organization/invites/{}", enc(invite_id)),
        )
    }
}

impl AdminOrganizationUsersResource {
    /// Retrieves an organization user.
    pub fn retrieve(&self, user_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.users.retrieve",
            Method::GET,
            format!("/organization/users/{}", enc(user_id)),
        )
    }

    /// Updates an organization user.
    pub fn update(&self, user_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.users.update",
            Method::POST,
            format!("/organization/users/{}", enc(user_id)),
        )
    }

    /// Lists organization users.
    pub fn list(&self) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.users.list",
            "/organization/users",
        )
    }

    /// Deletes an organization user.
    pub fn delete(&self, user_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.users.delete",
            Method::DELETE,
            format!("/organization/users/{}", enc(user_id)),
        )
    }

    /// Returns organization user role assignment resources.
    pub fn roles(&self) -> AdminOrganizationUserRolesResource {
        AdminOrganizationUserRolesResource::new(self.client.clone())
    }
}

impl AdminOrganizationUserRolesResource {
    /// Assigns an organization role to a user.
    pub fn create(&self, user_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.users.roles.create",
            Method::POST,
            format!("/organization/users/{}/roles", enc(user_id)),
        )
    }

    /// Retrieves an organization role assigned to a user.
    pub fn retrieve(
        &self,
        user_id: impl Into<String>,
        role_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.users.roles.retrieve",
            Method::GET,
            format!(
                "/organization/users/{}/roles/{}",
                enc(user_id),
                enc(role_id)
            ),
        )
    }

    /// Lists organization roles assigned to a user.
    pub fn list(&self, user_id: impl Into<String>) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.users.roles.list",
            format!("/organization/users/{}/roles", enc(user_id)),
        )
    }

    /// Unassigns an organization role from a user.
    pub fn delete(
        &self,
        user_id: impl Into<String>,
        role_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.users.roles.delete",
            Method::DELETE,
            format!(
                "/organization/users/{}/roles/{}",
                enc(user_id),
                enc(role_id)
            ),
        )
    }
}

impl AdminOrganizationGroupsResource {
    /// Creates an organization group.
    pub fn create(&self) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.groups.create",
            Method::POST,
            "/organization/groups",
        )
    }

    /// Retrieves an organization group.
    pub fn retrieve(&self, group_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.groups.retrieve",
            Method::GET,
            format!("/organization/groups/{}", enc(group_id)),
        )
    }

    /// Updates an organization group.
    pub fn update(&self, group_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.groups.update",
            Method::POST,
            format!("/organization/groups/{}", enc(group_id)),
        )
    }

    /// Lists organization groups.
    pub fn list(&self) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.groups.list",
            "/organization/groups",
        )
    }

    /// Deletes an organization group.
    pub fn delete(&self, group_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.groups.delete",
            Method::DELETE,
            format!("/organization/groups/{}", enc(group_id)),
        )
    }

    /// Returns organization group user assignment resources.
    pub fn users(&self) -> AdminOrganizationGroupUsersResource {
        AdminOrganizationGroupUsersResource::new(self.client.clone())
    }

    /// Returns organization group role assignment resources.
    pub fn roles(&self) -> AdminOrganizationGroupRolesResource {
        AdminOrganizationGroupRolesResource::new(self.client.clone())
    }
}

impl AdminOrganizationGroupUsersResource {
    /// Adds a user to an organization group.
    pub fn create(&self, group_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.groups.users.create",
            Method::POST,
            format!("/organization/groups/{}/users", enc(group_id)),
        )
    }

    /// Retrieves a user assigned to an organization group.
    pub fn retrieve(
        &self,
        group_id: impl Into<String>,
        user_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.groups.users.retrieve",
            Method::GET,
            format!(
                "/organization/groups/{}/users/{}",
                enc(group_id),
                enc(user_id)
            ),
        )
    }

    /// Lists users in an organization group.
    pub fn list(&self, group_id: impl Into<String>) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.groups.users.list",
            format!("/organization/groups/{}/users", enc(group_id)),
        )
    }

    /// Removes a user from an organization group.
    pub fn delete(
        &self,
        group_id: impl Into<String>,
        user_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.groups.users.delete",
            Method::DELETE,
            format!(
                "/organization/groups/{}/users/{}",
                enc(group_id),
                enc(user_id)
            ),
        )
    }
}

impl AdminOrganizationGroupRolesResource {
    /// Assigns an organization role to a group.
    pub fn create(&self, group_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.groups.roles.create",
            Method::POST,
            format!("/organization/groups/{}/roles", enc(group_id)),
        )
    }

    /// Retrieves an organization role assigned to a group.
    pub fn retrieve(
        &self,
        group_id: impl Into<String>,
        role_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.groups.roles.retrieve",
            Method::GET,
            format!(
                "/organization/groups/{}/roles/{}",
                enc(group_id),
                enc(role_id)
            ),
        )
    }

    /// Lists organization roles assigned to a group.
    pub fn list(&self, group_id: impl Into<String>) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.groups.roles.list",
            format!("/organization/groups/{}/roles", enc(group_id)),
        )
    }

    /// Unassigns an organization role from a group.
    pub fn delete(
        &self,
        group_id: impl Into<String>,
        role_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.groups.roles.delete",
            Method::DELETE,
            format!(
                "/organization/groups/{}/roles/{}",
                enc(group_id),
                enc(role_id)
            ),
        )
    }
}

impl AdminOrganizationRolesResource {
    /// Creates an organization role.
    pub fn create(&self) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.roles.create",
            Method::POST,
            "/organization/roles",
        )
    }

    /// Retrieves an organization role.
    pub fn retrieve(&self, role_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.roles.retrieve",
            Method::GET,
            format!("/organization/roles/{}", enc(role_id)),
        )
    }

    /// Updates an organization role.
    pub fn update(&self, role_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.roles.update",
            Method::POST,
            format!("/organization/roles/{}", enc(role_id)),
        )
    }

    /// Lists organization roles.
    pub fn list(&self) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.roles.list",
            "/organization/roles",
        )
    }

    /// Deletes an organization role.
    pub fn delete(&self, role_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.roles.delete",
            Method::DELETE,
            format!("/organization/roles/{}", enc(role_id)),
        )
    }
}

impl AdminOrganizationDataRetentionResource {
    /// Retrieves organization data retention settings.
    pub fn retrieve(&self) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.data_retention.retrieve",
            Method::GET,
            "/organization/data_retention",
        )
    }

    /// Updates organization data retention settings.
    pub fn update(&self) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.data_retention.update",
            Method::POST,
            "/organization/data_retention",
        )
    }
}

impl AdminOrganizationSpendAlertsResource {
    /// Creates an organization spend alert.
    pub fn create(&self) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.spend_alerts.create",
            Method::POST,
            "/organization/spend_alerts",
        )
    }

    /// Updates an organization spend alert.
    pub fn update(&self, alert_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.spend_alerts.update",
            Method::POST,
            format!("/organization/spend_alerts/{}", enc(alert_id)),
        )
    }

    /// Lists organization spend alerts.
    pub fn list(&self) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.spend_alerts.list",
            "/organization/spend_alerts",
        )
    }

    /// Deletes an organization spend alert.
    pub fn delete(&self, alert_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.spend_alerts.delete",
            Method::DELETE,
            format!("/organization/spend_alerts/{}", enc(alert_id)),
        )
    }
}

impl AdminOrganizationCertificatesResource {
    /// Creates an organization certificate.
    pub fn create(&self) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.certificates.create",
            Method::POST,
            "/organization/certificates",
        )
    }

    /// Retrieves an organization certificate.
    pub fn retrieve(&self, certificate_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.certificates.retrieve",
            Method::GET,
            format!("/organization/certificates/{}", enc(certificate_id)),
        )
    }

    /// Updates an organization certificate.
    pub fn update(&self, certificate_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.certificates.update",
            Method::POST,
            format!("/organization/certificates/{}", enc(certificate_id)),
        )
    }

    /// Lists organization certificates.
    pub fn list(&self) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.certificates.list",
            "/organization/certificates",
        )
    }

    /// Deletes an organization certificate.
    pub fn delete(&self, certificate_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.certificates.delete",
            Method::DELETE,
            format!("/organization/certificates/{}", enc(certificate_id)),
        )
    }

    /// Activates organization certificates.
    pub fn activate(&self) -> JsonRequestBuilder<Page<Value>> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.certificates.activate",
            Method::POST,
            "/organization/certificates/activate",
        )
    }

    /// Deactivates organization certificates.
    pub fn deactivate(&self) -> JsonRequestBuilder<Page<Value>> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.certificates.deactivate",
            Method::POST,
            "/organization/certificates/deactivate",
        )
    }
}

impl AdminOrganizationProjectsResource {
    /// Creates an organization project.
    pub fn create(&self) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.create",
            Method::POST,
            "/organization/projects",
        )
    }

    /// Retrieves an organization project.
    pub fn retrieve(&self, project_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.retrieve",
            Method::GET,
            format!("/organization/projects/{}", enc(project_id)),
        )
    }

    /// Updates an organization project.
    pub fn update(&self, project_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.update",
            Method::POST,
            format!("/organization/projects/{}", enc(project_id)),
        )
    }

    /// Lists organization projects.
    pub fn list(&self) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.list",
            "/organization/projects",
        )
    }

    /// Archives an organization project.
    pub fn archive(&self, project_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.archive",
            Method::POST,
            format!("/organization/projects/{}/archive", enc(project_id)),
        )
    }

    /// Returns project user resources.
    pub fn users(&self) -> AdminProjectUsersResource {
        AdminProjectUsersResource::new(self.client.clone())
    }

    /// Returns project service account resources.
    pub fn service_accounts(&self) -> AdminProjectServiceAccountsResource {
        AdminProjectServiceAccountsResource::new(self.client.clone())
    }

    /// Returns project API key resources.
    pub fn api_keys(&self) -> AdminProjectApiKeysResource {
        AdminProjectApiKeysResource::new(self.client.clone())
    }

    /// Returns project rate limit resources.
    pub fn rate_limits(&self) -> AdminProjectRateLimitsResource {
        AdminProjectRateLimitsResource::new(self.client.clone())
    }

    /// Returns project model permission resources.
    pub fn model_permissions(&self) -> AdminProjectModelPermissionsResource {
        AdminProjectModelPermissionsResource::new(self.client.clone())
    }

    /// Returns project hosted tool permission resources.
    pub fn hosted_tool_permissions(&self) -> AdminProjectHostedToolPermissionsResource {
        AdminProjectHostedToolPermissionsResource::new(self.client.clone())
    }

    /// Returns project group resources.
    pub fn groups(&self) -> AdminProjectGroupsResource {
        AdminProjectGroupsResource::new(self.client.clone())
    }

    /// Returns project role resources.
    pub fn roles(&self) -> AdminProjectRolesResource {
        AdminProjectRolesResource::new(self.client.clone())
    }

    /// Returns project data retention resources.
    pub fn data_retention(&self) -> AdminProjectDataRetentionResource {
        AdminProjectDataRetentionResource::new(self.client.clone())
    }

    /// Returns project spend alert resources.
    pub fn spend_alerts(&self) -> AdminProjectSpendAlertsResource {
        AdminProjectSpendAlertsResource::new(self.client.clone())
    }

    /// Returns project certificate resources.
    pub fn certificates(&self) -> AdminProjectCertificatesResource {
        AdminProjectCertificatesResource::new(self.client.clone())
    }
}

impl AdminProjectApiKeysResource {
    /// Retrieves a project API key.
    pub fn retrieve(
        &self,
        project_id: impl Into<String>,
        api_key_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.api_keys.retrieve",
            Method::GET,
            format!(
                "/organization/projects/{}/api_keys/{}",
                enc(project_id),
                enc(api_key_id)
            ),
        )
    }

    /// Lists project API keys.
    pub fn list(&self, project_id: impl Into<String>) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.api_keys.list",
            format!("/organization/projects/{}/api_keys", enc(project_id)),
        )
    }

    /// Deletes a project API key.
    pub fn delete(
        &self,
        project_id: impl Into<String>,
        api_key_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.api_keys.delete",
            Method::DELETE,
            format!(
                "/organization/projects/{}/api_keys/{}",
                enc(project_id),
                enc(api_key_id)
            ),
        )
    }
}

impl AdminProjectRateLimitsResource {
    /// Lists project rate limits.
    pub fn list_rate_limits(&self, project_id: impl Into<String>) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.rate_limits.list",
            format!("/organization/projects/{}/rate_limits", enc(project_id)),
        )
    }

    /// Updates a project rate limit.
    pub fn update_rate_limit(
        &self,
        project_id: impl Into<String>,
        rate_limit_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.rate_limits.update",
            Method::POST,
            format!(
                "/organization/projects/{}/rate_limits/{}",
                enc(project_id),
                enc(rate_limit_id)
            ),
        )
    }
}

impl AdminProjectServiceAccountsResource {
    /// Creates a project service account.
    pub fn create(&self, project_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.service_accounts.create",
            Method::POST,
            format!(
                "/organization/projects/{}/service_accounts",
                enc(project_id)
            ),
        )
    }

    /// Retrieves a project service account.
    pub fn retrieve(
        &self,
        project_id: impl Into<String>,
        service_account_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.service_accounts.retrieve",
            Method::GET,
            format!(
                "/organization/projects/{}/service_accounts/{}",
                enc(project_id),
                enc(service_account_id)
            ),
        )
    }

    /// Updates a project service account.
    pub fn update(
        &self,
        project_id: impl Into<String>,
        service_account_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.service_accounts.update",
            Method::POST,
            format!(
                "/organization/projects/{}/service_accounts/{}",
                enc(project_id),
                enc(service_account_id)
            ),
        )
    }

    /// Lists project service accounts.
    pub fn list(&self, project_id: impl Into<String>) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.service_accounts.list",
            format!(
                "/organization/projects/{}/service_accounts",
                enc(project_id)
            ),
        )
    }

    /// Deletes a project service account.
    pub fn delete(
        &self,
        project_id: impl Into<String>,
        service_account_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.service_accounts.delete",
            Method::DELETE,
            format!(
                "/organization/projects/{}/service_accounts/{}",
                enc(project_id),
                enc(service_account_id)
            ),
        )
    }
}

impl AdminProjectUsersResource {
    /// Adds a user to a project.
    pub fn create(&self, project_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.users.create",
            Method::POST,
            format!("/organization/projects/{}/users", enc(project_id)),
        )
    }

    /// Retrieves a project user.
    pub fn retrieve(
        &self,
        project_id: impl Into<String>,
        user_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.users.retrieve",
            Method::GET,
            format!(
                "/organization/projects/{}/users/{}",
                enc(project_id),
                enc(user_id)
            ),
        )
    }

    /// Updates a project user.
    pub fn update(
        &self,
        project_id: impl Into<String>,
        user_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.users.update",
            Method::POST,
            format!(
                "/organization/projects/{}/users/{}",
                enc(project_id),
                enc(user_id)
            ),
        )
    }

    /// Lists project users.
    pub fn list(&self, project_id: impl Into<String>) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.users.list",
            format!("/organization/projects/{}/users", enc(project_id)),
        )
    }

    /// Removes a user from a project.
    pub fn delete(
        &self,
        project_id: impl Into<String>,
        user_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.users.delete",
            Method::DELETE,
            format!(
                "/organization/projects/{}/users/{}",
                enc(project_id),
                enc(user_id)
            ),
        )
    }

    /// Returns project user role assignment resources.
    pub fn roles(&self) -> AdminProjectUserRolesResource {
        AdminProjectUserRolesResource::new(self.client.clone())
    }
}

impl AdminProjectUserRolesResource {
    /// Assigns a project role to a user.
    pub fn create(
        &self,
        project_id: impl Into<String>,
        user_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.users.roles.create",
            Method::POST,
            format!("/projects/{}/users/{}/roles", enc(project_id), enc(user_id)),
        )
    }

    /// Retrieves a project role assigned to a user.
    pub fn retrieve(
        &self,
        project_id: impl Into<String>,
        user_id: impl Into<String>,
        role_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.users.roles.retrieve",
            Method::GET,
            format!(
                "/projects/{}/users/{}/roles/{}",
                enc(project_id),
                enc(user_id),
                enc(role_id)
            ),
        )
    }

    /// Lists project roles assigned to a user.
    pub fn list(
        &self,
        project_id: impl Into<String>,
        user_id: impl Into<String>,
    ) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.users.roles.list",
            format!("/projects/{}/users/{}/roles", enc(project_id), enc(user_id)),
        )
    }

    /// Unassigns a project role from a user.
    pub fn delete(
        &self,
        project_id: impl Into<String>,
        user_id: impl Into<String>,
        role_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.users.roles.delete",
            Method::DELETE,
            format!(
                "/projects/{}/users/{}/roles/{}",
                enc(project_id),
                enc(user_id),
                enc(role_id)
            ),
        )
    }
}

impl AdminProjectGroupsResource {
    /// Grants a group access to a project.
    pub fn create(&self, project_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.groups.create",
            Method::POST,
            format!("/organization/projects/{}/groups", enc(project_id)),
        )
    }

    /// Retrieves a project group.
    pub fn retrieve(
        &self,
        project_id: impl Into<String>,
        group_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.groups.retrieve",
            Method::GET,
            format!(
                "/organization/projects/{}/groups/{}",
                enc(project_id),
                enc(group_id)
            ),
        )
    }

    /// Lists groups that have access to a project.
    pub fn list(&self, project_id: impl Into<String>) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.groups.list",
            format!("/organization/projects/{}/groups", enc(project_id)),
        )
    }

    /// Revokes a group's access to a project.
    pub fn delete(
        &self,
        project_id: impl Into<String>,
        group_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.groups.delete",
            Method::DELETE,
            format!(
                "/organization/projects/{}/groups/{}",
                enc(project_id),
                enc(group_id)
            ),
        )
    }

    /// Returns project group role assignment resources.
    pub fn roles(&self) -> AdminProjectGroupRolesResource {
        AdminProjectGroupRolesResource::new(self.client.clone())
    }
}

impl AdminProjectGroupRolesResource {
    /// Assigns a project role to a group.
    pub fn create(
        &self,
        project_id: impl Into<String>,
        group_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.groups.roles.create",
            Method::POST,
            format!(
                "/projects/{}/groups/{}/roles",
                enc(project_id),
                enc(group_id)
            ),
        )
    }

    /// Retrieves a project role assigned to a group.
    pub fn retrieve(
        &self,
        project_id: impl Into<String>,
        group_id: impl Into<String>,
        role_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.groups.roles.retrieve",
            Method::GET,
            format!(
                "/projects/{}/groups/{}/roles/{}",
                enc(project_id),
                enc(group_id),
                enc(role_id)
            ),
        )
    }

    /// Lists project roles assigned to a group.
    pub fn list(
        &self,
        project_id: impl Into<String>,
        group_id: impl Into<String>,
    ) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.groups.roles.list",
            format!(
                "/projects/{}/groups/{}/roles",
                enc(project_id),
                enc(group_id)
            ),
        )
    }

    /// Unassigns a project role from a group.
    pub fn delete(
        &self,
        project_id: impl Into<String>,
        group_id: impl Into<String>,
        role_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.groups.roles.delete",
            Method::DELETE,
            format!(
                "/projects/{}/groups/{}/roles/{}",
                enc(project_id),
                enc(group_id),
                enc(role_id)
            ),
        )
    }
}

impl AdminProjectRolesResource {
    /// Creates a custom project role.
    pub fn create(&self, project_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.roles.create",
            Method::POST,
            format!("/projects/{}/roles", enc(project_id)),
        )
    }

    /// Retrieves a project role.
    pub fn retrieve(
        &self,
        project_id: impl Into<String>,
        role_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.roles.retrieve",
            Method::GET,
            format!("/projects/{}/roles/{}", enc(project_id), enc(role_id)),
        )
    }

    /// Updates a project role.
    pub fn update(
        &self,
        project_id: impl Into<String>,
        role_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.roles.update",
            Method::POST,
            format!("/projects/{}/roles/{}", enc(project_id), enc(role_id)),
        )
    }

    /// Lists project roles.
    pub fn list(&self, project_id: impl Into<String>) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.roles.list",
            format!("/projects/{}/roles", enc(project_id)),
        )
    }

    /// Deletes a project role.
    pub fn delete(
        &self,
        project_id: impl Into<String>,
        role_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.roles.delete",
            Method::DELETE,
            format!("/projects/{}/roles/{}", enc(project_id), enc(role_id)),
        )
    }
}

impl AdminProjectDataRetentionResource {
    /// Retrieves project data retention settings.
    pub fn retrieve(&self, project_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.data_retention.retrieve",
            Method::GET,
            format!("/organization/projects/{}/data_retention", enc(project_id)),
        )
    }

    /// Updates project data retention settings.
    pub fn update(&self, project_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.data_retention.update",
            Method::POST,
            format!("/organization/projects/{}/data_retention", enc(project_id)),
        )
    }
}

impl AdminProjectSpendAlertsResource {
    /// Creates a project spend alert.
    pub fn create(&self, project_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.spend_alerts.create",
            Method::POST,
            format!("/organization/projects/{}/spend_alerts", enc(project_id)),
        )
    }

    /// Updates a project spend alert.
    pub fn update(
        &self,
        project_id: impl Into<String>,
        alert_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.spend_alerts.update",
            Method::POST,
            format!(
                "/organization/projects/{}/spend_alerts/{}",
                enc(project_id),
                enc(alert_id)
            ),
        )
    }

    /// Lists project spend alerts.
    pub fn list(&self, project_id: impl Into<String>) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.spend_alerts.list",
            format!("/organization/projects/{}/spend_alerts", enc(project_id)),
        )
    }

    /// Deletes a project spend alert.
    pub fn delete(
        &self,
        project_id: impl Into<String>,
        alert_id: impl Into<String>,
    ) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.spend_alerts.delete",
            Method::DELETE,
            format!(
                "/organization/projects/{}/spend_alerts/{}",
                enc(project_id),
                enc(alert_id)
            ),
        )
    }
}

impl AdminProjectCertificatesResource {
    /// Lists project certificates.
    pub fn list(&self, project_id: impl Into<String>) -> ListRequestBuilder<Value> {
        ListRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.certificates.list",
            format!("/organization/projects/{}/certificates", enc(project_id)),
        )
    }

    /// Activates project certificates.
    pub fn activate(&self, project_id: impl Into<String>) -> JsonRequestBuilder<Page<Value>> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.certificates.activate",
            Method::POST,
            format!(
                "/organization/projects/{}/certificates/activate",
                enc(project_id)
            ),
        )
    }

    /// Deactivates project certificates.
    pub fn deactivate(&self, project_id: impl Into<String>) -> JsonRequestBuilder<Page<Value>> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.certificates.deactivate",
            Method::POST,
            format!(
                "/organization/projects/{}/certificates/deactivate",
                enc(project_id)
            ),
        )
    }
}

impl AdminProjectModelPermissionsResource {
    /// Retrieves project model permissions.
    pub fn retrieve(&self, project_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.model_permissions.retrieve",
            Method::GET,
            format!(
                "/organization/projects/{}/model_permissions",
                enc(project_id)
            ),
        )
    }

    /// Updates project model permissions.
    pub fn update(&self, project_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.model_permissions.update",
            Method::POST,
            format!(
                "/organization/projects/{}/model_permissions",
                enc(project_id)
            ),
        )
    }

    /// Deletes project model permissions.
    pub fn delete(&self, project_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.model_permissions.delete",
            Method::DELETE,
            format!(
                "/organization/projects/{}/model_permissions",
                enc(project_id)
            ),
        )
    }
}

impl AdminProjectHostedToolPermissionsResource {
    /// Retrieves project hosted tool permissions.
    pub fn retrieve(&self, project_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.hosted_tool_permissions.retrieve",
            Method::GET,
            format!(
                "/organization/projects/{}/hosted_tool_permissions",
                enc(project_id)
            ),
        )
    }

    /// Updates project hosted tool permissions.
    pub fn update(&self, project_id: impl Into<String>) -> JsonRequestBuilder<Value> {
        JsonRequestBuilder::new(
            self.client.clone(),
            "admin.organization.projects.hosted_tool_permissions.update",
            Method::POST,
            format!(
                "/organization/projects/{}/hosted_tool_permissions",
                enc(project_id)
            ),
        )
    }
}
