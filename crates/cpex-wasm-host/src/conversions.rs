// Location: ./crates/cpex-wasm-host/src/conversions.rs
// Copyright 2025
// SPDX-License-Identifier: Apache-2.0
// Authors: Shriti Priya
//
// Host-side type conversions: native cpex-core types ↔ WIT types.
// Used by WasmBridgeHandler to translate between the PluginManager's native
// types and the WIT types that the WASM sandbox expects.

use std::collections::HashMap;

use cpex_core::cmf::content as native_content;
use cpex_core::cmf::enums as native_enums;
use cpex_core::cmf::message as native_msg;
use cpex_core::context::PluginContext as NativePluginContext;
use cpex_core::error::PluginViolation as NativePluginViolation;
use cpex_core::extensions::agent::AgentExtension as NativeAgentExtension;
use cpex_core::extensions::completion::{
    CompletionExtension as NativeCompletionExtension, StopReason as NativeStopReason,
};
use cpex_core::extensions::container::{Extensions as NativeExtensions, OwnedExtensions as NativeOwnedExtensions};
use cpex_core::extensions::delegation::{
    DelegationExtension as NativeDelegationExtension,
    DelegationStrategy as NativeDelegationStrategy,
};
use cpex_core::extensions::framework::FrameworkExtension as NativeFrameworkExtension;
use cpex_core::extensions::guarded::Guarded;
use cpex_core::extensions::http::HttpExtension as NativeHttpExtension;
use cpex_core::extensions::llm::LLMExtension as NativeLLMExtension;
use cpex_core::extensions::mcp::MCPExtension as NativeMCPExtension;
use cpex_core::extensions::meta::MetaExtension as NativeMetaExtension;
use cpex_core::extensions::monotonic::MonotonicSet;
use cpex_core::extensions::provenance::ProvenanceExtension as NativeProvenanceExtension;
use cpex_core::extensions::request::RequestExtension as NativeRequestExtension;
use cpex_core::extensions::security::{
    ClientExtension as NativeClientExtension, ClientTrustLevel as NativeClientTrustLevel,
    DataPolicy as NativeDataPolicy,
    ObjectSecurityProfile as NativeObjectSecurityProfile,
    RetentionPolicy as NativeRetentionPolicy,
    SecurityExtension as NativeSecurityExtension, SubjectExtension as NativeSubjectExtension,
    SubjectType as NativeSubjectType, WorkloadIdentity as NativeWorkloadIdentity,
};
use cpex_core::hooks::payload::PluginPayload as NativePluginPayload;
use cpex_core::hooks::trait_def::PluginResult as NativePluginResult;

use crate::sandbox_manager::types::*;

// ---------------------------------------------------------------------------
// Native → WIT: Payload (variant — message or custom)
// ---------------------------------------------------------------------------

/// Converts any native PluginPayload to the WIT payload variant.
///
/// If the payload is a `MessagePayload`, it is converted to the structured
/// `Payload::Message` arm. Any other payload type is serialized to JSON
/// and wrapped in `Payload::Custom`. Returns an error if a non-MessagePayload
/// type doesn't implement `to_json()`.
pub fn native_any_payload_to_wit(
    payload: &dyn NativePluginPayload,
) -> Result<Payload, String> {
    if let Some(mp) = payload.as_any().downcast_ref::<native_msg::MessagePayload>() {
        Ok(Payload::Message(native_message_payload_to_wit(mp)))
    } else {
        let json = payload.to_json().ok_or_else(|| {
            format!(
                "payload type '{}' does not implement to_json() — \
                 use impl_plugin_payload_json! to enable WASM dispatch",
                payload.payload_type_name()
            )
        })?;
        Ok(Payload::Custom(CustomPayload {
            payload_type: payload.payload_type_name().to_string(),
            payload_json: json,
        }))
    }
}

/// Converts native cpex-core MessagePayload to WIT MessagePayload.
pub fn native_message_payload_to_wit(payload: &native_msg::MessagePayload) -> MessagePayload {
    MessagePayload {
        message: native_message_to_wit(&payload.message),
    }
}

fn native_message_to_wit(msg: &native_msg::Message) -> Message {
    Message {
        schema_version: msg.schema_version.clone(),
        role: native_role_to_wit(msg.role),
        content: msg.content.iter().map(native_content_part_to_wit).collect(),
        channel: msg.channel.map(native_channel_to_wit),
    }
}

fn native_role_to_wit(role: native_enums::Role) -> Role {
    match role {
        native_enums::Role::System => Role::System,
        native_enums::Role::Developer => Role::Developer,
        native_enums::Role::User => Role::User,
        native_enums::Role::Assistant => Role::Assistant,
        native_enums::Role::Tool => Role::Tool,
    }
}

fn native_channel_to_wit(channel: native_enums::Channel) -> Channel {
    match channel {
        native_enums::Channel::Analysis => Channel::Analysis,
        native_enums::Channel::Commentary => Channel::Commentary,
        native_enums::Channel::Final => Channel::Final,
    }
}

fn native_content_part_to_wit(part: &native_content::ContentPart) -> ContentPart {
    match part {
        native_content::ContentPart::Text { text } => ContentPart::Text(text.clone()),
        native_content::ContentPart::Thinking { text } => ContentPart::Thinking(text.clone()),
        native_content::ContentPart::ToolCall { content } => {
            ContentPart::ToolCall(native_tool_call_to_wit(content))
        }
        native_content::ContentPart::ToolResult { content } => {
            ContentPart::ToolResult(native_tool_result_to_wit(content))
        }
        native_content::ContentPart::Resource { content } => {
            ContentPart::CmfResource(native_resource_to_wit(content))
        }
        native_content::ContentPart::ResourceRef { content } => {
            ContentPart::ResourceRef(native_resource_ref_to_wit(content))
        }
        native_content::ContentPart::PromptRequest { content } => {
            ContentPart::PromptRequest(native_prompt_request_to_wit(content))
        }
        native_content::ContentPart::PromptResult { content } => {
            ContentPart::PromptResult(native_prompt_result_to_wit(content))
        }
        native_content::ContentPart::Image { content } => ContentPart::Image(ImageSource {
            source_type: content.source_type.clone(),
            data: content.data.clone(),
            media_type: content.media_type.clone(),
        }),
        native_content::ContentPart::Video { content } => ContentPart::Video(VideoSource {
            source_type: content.source_type.clone(),
            data: content.data.clone(),
            media_type: content.media_type.clone(),
            duration_ms: content.duration_ms,
        }),
        native_content::ContentPart::Audio { content } => ContentPart::Audio(AudioSource {
            source_type: content.source_type.clone(),
            data: content.data.clone(),
            media_type: content.media_type.clone(),
            duration_ms: content.duration_ms,
        }),
        native_content::ContentPart::Document { content } => {
            ContentPart::Document(DocumentSource {
                source_type: content.source_type.clone(),
                data: content.data.clone(),
                media_type: content.media_type.clone(),
                title: content.title.clone(),
            })
        }
    }
}

fn native_tool_call_to_wit(tc: &native_content::ToolCall) -> ToolCall {
    ToolCall {
        tool_call_id: tc.tool_call_id.clone(),
        name: tc.name.clone(),
        arguments: serde_json::to_string(&tc.arguments).unwrap_or_else(|_| "{}".to_string()),
        namespace: tc.namespace.clone(),
    }
}

fn native_tool_result_to_wit(tr: &native_content::ToolResult) -> ToolResult {
    ToolResult {
        tool_call_id: tr.tool_call_id.clone(),
        tool_name: tr.tool_name.clone(),
        content: serde_json::to_string(&tr.content).unwrap_or_default(),
        is_error: tr.is_error,
    }
}

fn native_resource_to_wit(r: &native_content::Resource) -> CmfResource {
    CmfResource {
        resource_request_id: r.resource_request_id.clone(),
        uri: r.uri.clone(),
        name: r.name.clone(),
        description: r.description.clone(),
        resource_type: native_resource_type_to_wit(r.resource_type),
        content: r.content.clone(),
        blob: r.blob.clone(),
        mime_type: r.mime_type.clone(),
        size_bytes: r.size_bytes,
        annotations: serde_json::to_string(&r.annotations).unwrap_or_else(|_| "{}".to_string()),
        version: r.version.clone(),
    }
}

fn native_resource_type_to_wit(rt: native_enums::ResourceType) -> ResourceType {
    match rt {
        native_enums::ResourceType::File => ResourceType::File,
        native_enums::ResourceType::Blob => ResourceType::Blob,
        native_enums::ResourceType::Uri => ResourceType::Uri,
        native_enums::ResourceType::Database => ResourceType::Database,
        native_enums::ResourceType::Api => ResourceType::Api,
        native_enums::ResourceType::Memory => ResourceType::Memory,
        native_enums::ResourceType::Artifact => ResourceType::Artifact,
    }
}

fn native_resource_ref_to_wit(rr: &native_content::ResourceReference) -> ResourceReference {
    ResourceReference {
        resource_request_id: rr.resource_request_id.clone(),
        uri: rr.uri.clone(),
        name: rr.name.clone(),
        resource_type: native_resource_type_to_wit(rr.resource_type),
        range_start: rr.range_start,
        range_end: rr.range_end,
        selector: rr.selector.clone(),
    }
}

fn native_prompt_request_to_wit(pr: &native_content::PromptRequest) -> PromptRequest {
    PromptRequest {
        prompt_request_id: pr.prompt_request_id.clone(),
        name: pr.name.clone(),
        arguments: serde_json::to_string(&pr.arguments).unwrap_or_else(|_| "{}".to_string()),
        server_id: pr.server_id.clone(),
    }
}

fn native_prompt_result_to_wit(pr: &native_content::PromptResult) -> PromptResult {
    PromptResult {
        prompt_request_id: pr.prompt_request_id.clone(),
        prompt_name: pr.prompt_name.clone(),
        messages: serde_json::to_string(&pr.messages).unwrap_or_else(|_| "[]".to_string()),
        content: pr.content.clone(),
        is_error: pr.is_error,
        error_message: pr.error_message.clone(),
    }
}

// ---------------------------------------------------------------------------
// Native → WIT: Extensions
// ---------------------------------------------------------------------------

/// Converts native cpex-core Extensions to WIT Extensions.
pub fn native_extensions_to_wit(ext: &NativeExtensions) -> Extensions {
    Extensions {
        request: ext.request.as_ref().map(|r| native_request_to_wit(r)),
        security: ext.security.as_ref().map(|s| native_security_to_wit(s)),
        http: ext.http.as_ref().map(|h| native_http_to_wit(h)),
        meta: ext.meta.as_ref().map(|m| native_meta_to_wit(m)),
        agent: ext.agent.as_ref().map(|a| native_agent_to_wit(a)),
        mcp: ext.mcp.as_ref().map(|m| native_mcp_to_wit(m)),
        completion: ext.completion.as_ref().map(|c| native_completion_to_wit(c)),
        provenance: ext.provenance.as_ref().map(|p| native_provenance_to_wit(p)),
        llm: ext.llm.as_ref().map(|l| native_llm_to_wit(l)),
        framework: ext.framework.as_ref().map(|f| native_framework_to_wit(f)),
        delegation: ext.delegation.as_ref().map(|d| native_delegation_to_wit(d)),
        custom: ext.custom.as_ref().and_then(|c| serde_json::to_string(c.as_ref()).ok()),
    }
}

fn native_request_to_wit(r: &NativeRequestExtension) -> RequestExtension {
    RequestExtension {
        environment: r.environment.clone(),
        request_id: r.request_id.clone(),
        timestamp: r.timestamp.clone(),
        trace_id: r.trace_id.clone(),
        span_id: r.span_id.clone(),
    }
}

fn native_security_to_wit(s: &NativeSecurityExtension) -> SecurityExtension {
    SecurityExtension {
        labels: s.labels.iter().cloned().collect(),
        classification: s.classification.clone(),
        subject: s.subject.as_ref().map(native_subject_to_wit),
        client: s.client.as_ref().map(native_client_to_wit),
        caller_workload: s.caller_workload.as_ref().map(native_workload_to_wit),
        this_workload: s.this_workload.as_ref().map(native_workload_to_wit),
        auth_method: s.auth_method.clone(),
        objects: s.objects.iter().map(|(k, v)| (k.clone(), ObjectSecurityProfile {
            managed_by: v.managed_by.clone(),
            permissions: v.permissions.clone(),
            trust_domain: v.trust_domain.clone(),
            data_scope: v.data_scope.clone(),
        })).collect(),
        data: s.data.iter().map(|(k, v)| (k.clone(), DataPolicy {
            apply_labels: v.apply_labels.clone(),
            allowed_actions: v.allowed_actions.clone(),
            denied_actions: v.denied_actions.clone(),
            retention: v.retention.as_ref().map(|r| RetentionPolicy {
                max_age_seconds: r.max_age_seconds,
                policy: r.policy.clone(),
                delete_after: r.delete_after.clone(),
            }),
        })).collect(),
    }
}

fn native_client_to_wit(c: &NativeClientExtension) -> ClientExtension {
    ClientExtension {
        client_id: c.client_id.clone(),
        client_name: c.client_name.clone(),
        trust_level: match &c.trust_level {
            NativeClientTrustLevel::FirstParty => ClientTrustLevel::FirstParty,
            NativeClientTrustLevel::Internal => ClientTrustLevel::Internal,
            _ => ClientTrustLevel::ThirdParty,
        },
        authorized_scopes: c.authorized_scopes.clone(),
        authorized_audiences: c.authorized_audiences.clone(),
        roles: c.roles.clone(),
        permissions: c.permissions.clone(),
        teams: c.teams.clone(),
        claims: c.claims.iter().map(|(k, v)| (k.clone(), v.to_string())).collect(),
    }
}

fn native_workload_to_wit(w: &NativeWorkloadIdentity) -> WorkloadIdentity {
    WorkloadIdentity {
        spiffe_id: w.spiffe_id.clone(),
        trust_domain: w.trust_domain.clone(),
        attested_at: w.attested_at.map(|t| t.to_rfc3339()),
        attestor: w.attestor.clone(),
        selectors: w.selectors.clone(),
        client_id: w.client_id.clone(),
    }
}

fn native_subject_to_wit(s: &NativeSubjectExtension) -> SubjectExtension {
    SubjectExtension {
        id: s.id.clone(),
        subject_type: s.subject_type.as_ref().map(native_subject_type_to_wit),
        roles: s.roles.iter().cloned().collect(),
        permissions: s.permissions.iter().cloned().collect(),
        teams: s.teams.iter().cloned().collect(),
        claims: s.claims.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
    }
}

fn native_subject_type_to_wit(st: &NativeSubjectType) -> SubjectType {
    match st {
        NativeSubjectType::User => SubjectType::User,
        NativeSubjectType::Agent => SubjectType::Agent,
        NativeSubjectType::Service => SubjectType::Service,
        NativeSubjectType::System => SubjectType::System,
    }
}

fn native_http_to_wit(h: &NativeHttpExtension) -> HttpExtension {
    HttpExtension {
        request_headers: h.request_headers.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        response_headers: h.response_headers.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
    }
}

fn native_meta_to_wit(m: &NativeMetaExtension) -> MetaExtension {
    MetaExtension {
        entity_type: m.entity_type.clone(),
        entity_name: m.entity_name.clone(),
        tags: m.tags.iter().cloned().collect(),
        scope: m.scope.clone(),
        properties: m.properties.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
    }
}

fn native_agent_to_wit(a: &NativeAgentExtension) -> AgentExtension {
    AgentExtension {
        input: a.input.clone(),
        session_id: a.session_id.clone(),
        conversation_id: a.conversation_id.clone(),
        turn: a.turn,
        agent_id: a.agent_id.clone(),
        parent_agent_id: a.parent_agent_id.clone(),
        conversation: a.conversation.as_ref().map(|c| ConversationContext {
            history: serde_json::to_string(&c.history).unwrap_or_else(|_| "[]".to_string()),
            summary: c.summary.clone(),
            topics: c.topics.clone(),
        }),
    }
}

fn native_mcp_to_wit(m: &NativeMCPExtension) -> McpExtension {
    McpExtension {
        tool: m.tool.as_ref().map(|t| ToolMetadata {
            name: t.name.clone(),
            title: t.title.clone(),
            description: t.description.clone(),
            input_schema: t.input_schema.as_ref().and_then(|s| serde_json::to_string(s).ok()),
            output_schema: t.output_schema.as_ref().and_then(|s| serde_json::to_string(s).ok()),
            server_id: t.server_id.clone(),
            namespace: t.namespace.clone(),
            annotations: t.annotations.iter()
                .map(|(k, v)| (k.clone(), v.to_string()))
                .collect(),
        }),
        mcp_resource: m.resource.as_ref().map(|r| ResourceMetadata {
            uri: r.uri.clone(),
            name: r.name.clone(),
            description: r.description.clone(),
            mime_type: r.mime_type.clone(),
            server_id: r.server_id.clone(),
            annotations: r.annotations.iter()
                .map(|(k, v)| (k.clone(), v.to_string()))
                .collect(),
        }),
        mcp_prompt: m.prompt.as_ref().map(|p| PromptMetadata {
            name: p.name.clone(),
            description: p.description.clone(),
            arguments: p.arguments.as_ref().and_then(|a| serde_json::to_string(a).ok()),
            server_id: p.server_id.clone(),
            annotations: p.annotations.iter()
                .map(|(k, v)| (k.clone(), v.to_string()))
                .collect(),
        }),
    }
}

fn native_completion_to_wit(c: &NativeCompletionExtension) -> CompletionExtension {
    CompletionExtension {
        stop_reason: c.stop_reason.map(|r| match r {
            NativeStopReason::End => StopReason::End,
            NativeStopReason::Return => StopReason::Return,
            NativeStopReason::Call => StopReason::Call,
            NativeStopReason::MaxTokens => StopReason::MaxTokens,
            NativeStopReason::StopSequence => StopReason::StopSequence,
        }),
        tokens: c.tokens.as_ref().map(|t| TokenUsage {
            input_tokens: t.input_tokens,
            output_tokens: t.output_tokens,
            total_tokens: t.total_tokens,
        }),
        model: c.model.clone(),
        raw_format: c.raw_format.clone(),
        created_at: c.created_at.clone(),
        latency_ms: c.latency_ms,
    }
}

fn native_provenance_to_wit(p: &NativeProvenanceExtension) -> ProvenanceExtension {
    ProvenanceExtension {
        source: p.source.clone(),
        message_id: p.message_id.clone(),
        parent_id: p.parent_id.clone(),
    }
}

fn native_llm_to_wit(l: &NativeLLMExtension) -> LlmExtension {
    LlmExtension {
        model_id: l.model_id.clone(),
        provider: l.provider.clone(),
        capabilities: l.capabilities.clone(),
    }
}

fn native_framework_to_wit(f: &NativeFrameworkExtension) -> FrameworkExtension {
    FrameworkExtension {
        framework: f.framework.clone(),
        framework_version: f.framework_version.clone(),
        node_id: f.node_id.clone(),
        graph_id: f.graph_id.clone(),
        metadata: f.metadata.iter()
            .map(|(k, v)| (k.clone(), v.to_string()))
            .collect(),
    }
}

fn native_delegation_to_wit(d: &NativeDelegationExtension) -> DelegationExtension {
    DelegationExtension {
        chain: d.chain.iter().map(|h| DelegationHop {
            subject_id: h.subject_id.clone(),
            subject_type: h.subject_type.as_ref().map(|st| format!("{:?}", st).to_lowercase()),
            audience: h.audience.clone(),
            scopes_granted: h.scopes_granted.clone(),
            timestamp: Some(h.timestamp.to_rfc3339()),
            ttl_seconds: h.ttl_seconds,
            strategy: h.strategy.as_ref().map(|s| match s {
                NativeDelegationStrategy::TokenExchange => "token_exchange".to_string(),
                NativeDelegationStrategy::Custom(v) => v.clone(),
                _ => "unknown".to_string(),
            }),
            from_cache: h.from_cache,
        }).collect(),
        depth: d.depth as u32,
        origin_subject_id: d.origin_subject_id.clone(),
        actor_subject_id: d.actor_subject_id.clone(),
        delegated: d.delegated,
        age_seconds: d.age_seconds,
    }
}


// ---------------------------------------------------------------------------
// Native → WIT: PluginContext
// ---------------------------------------------------------------------------

/// Converts native PluginContext (HashMaps) to WIT PluginContext (JSON strings).
pub fn native_context_to_wit(ctx: &NativePluginContext) -> PluginContext {
    PluginContext {
        local_state: serde_json::to_string(&ctx.local_state).unwrap_or_else(|_| "{}".to_string()),
        global_state: serde_json::to_string(&ctx.global_state).unwrap_or_else(|_| "{}".to_string()),
    }
}

// ---------------------------------------------------------------------------
// WIT → Native: PluginResult
// ---------------------------------------------------------------------------

/// Converts a WIT PluginResult from the sandbox back to native PluginResult.
/// Requires the original extensions to preserve immutable Arc pointers for executor validation.
///
/// `modified_payload` in the WIT result is a `payload` variant — either a structured
/// `MessagePayload` or a JSON-encoded custom payload. Both are returned as
/// `Box<dyn PluginPayload>` so the executor can handle them uniformly.
pub fn wit_result_to_native(
    result: crate::sandbox_manager::types::PluginResult,
    original_extensions: &NativeExtensions,
) -> NativePluginResult<native_msg::MessagePayload> {
    NativePluginResult {
        continue_processing: result.continue_processing,
        modified_payload: result.modified_payload.and_then(wit_payload_variant_to_native),
        modified_extensions: result.modified_extensions.map(|ext| {
            wit_extensions_to_owned_native(ext, original_extensions)
        }),
        violation: result.violation.map(wit_violation_to_native),
        metadata: result.metadata.and_then(|s| serde_json::from_str(&s).ok()),
    }
}

/// Converts WIT Extensions into a native OwnedExtensions.
/// Immutable slots are carried over from the original (preserving Arc pointer identity).
/// Mutable slots (security, http) are rebuilt from the WIT data.
fn wit_extensions_to_owned_native(
    wit_ext: Extensions,
    original: &NativeExtensions,
) -> NativeOwnedExtensions {
    NativeOwnedExtensions {
        // Immutable — preserve original Arc pointers for ptr_eq validation
        request: original.request.clone(),
        agent: original.agent.clone(),
        mcp: original.mcp.clone(),
        completion: original.completion.clone(),
        provenance: original.provenance.clone(),
        llm: original.llm.clone(),
        framework: original.framework.clone(),
        meta: original.meta.clone(),
        raw_credentials: original.raw_credentials.clone(),

        // Mutable — rebuild from WIT data
        http: wit_ext.http.map(|h| {
            Guarded::new(NativeHttpExtension {
                request_headers: h.request_headers.into_iter().collect(),
                response_headers: h.response_headers.into_iter().collect(),
            })
        }),
        security: wit_ext.security.map(|s| wit_security_to_native_owned(s)),
        delegation: None,
        custom: None,

        // Write tokens — not applicable across the WASM boundary
        http_write_token: None,
        labels_write_token: None,
        delegation_write_token: None,
    }
}

/// Converts WIT SecurityExtension to a native owned SecurityExtension (not wrapped in Arc).
fn wit_security_to_native_owned(s: SecurityExtension) -> NativeSecurityExtension {
    NativeSecurityExtension {
        labels: MonotonicSet::from_set(s.labels.into_iter().collect()),
        classification: s.classification,
        subject: s.subject.map(|sub| NativeSubjectExtension {
            id: sub.id,
            subject_type: sub.subject_type.map(|st| match st {
                SubjectType::User => NativeSubjectType::User,
                SubjectType::Agent => NativeSubjectType::Agent,
                SubjectType::Service => NativeSubjectType::Service,
                SubjectType::System => NativeSubjectType::System,
            }),
            roles: sub.roles.into_iter().collect(),
            permissions: sub.permissions.into_iter().collect(),
            teams: sub.teams.into_iter().collect(),
            claims: sub.claims.into_iter().collect(),
        }),
        client: s.client.map(|c| NativeClientExtension {
            client_id: c.client_id,
            client_name: c.client_name,
            trust_level: match c.trust_level {
                ClientTrustLevel::FirstParty => NativeClientTrustLevel::FirstParty,
                ClientTrustLevel::Internal => NativeClientTrustLevel::Internal,
                ClientTrustLevel::ThirdParty => NativeClientTrustLevel::ThirdParty,
            },
            authorized_scopes: c.authorized_scopes,
            authorized_audiences: c.authorized_audiences,
            roles: c.roles,
            permissions: c.permissions,
            teams: c.teams,
            claims: c.claims.into_iter()
                .map(|(k, v)| (k, serde_json::Value::String(v)))
                .collect(),
        }),
        caller_workload: s.caller_workload.map(wit_workload_to_native),
        this_workload: s.this_workload.map(wit_workload_to_native),
        auth_method: s.auth_method,
        objects: s.objects.into_iter().map(|(k, v)| (k, NativeObjectSecurityProfile {
            managed_by: v.managed_by,
            permissions: v.permissions,
            trust_domain: v.trust_domain,
            data_scope: v.data_scope,
        })).collect(),
        data: s.data.into_iter().map(|(k, v)| (k, NativeDataPolicy {
            apply_labels: v.apply_labels,
            allowed_actions: v.allowed_actions,
            denied_actions: v.denied_actions,
            retention: v.retention.map(|r| NativeRetentionPolicy {
                max_age_seconds: r.max_age_seconds,
                policy: r.policy,
                delete_after: r.delete_after,
            }),
        })).collect(),
    }
}

fn wit_workload_to_native(w: WorkloadIdentity) -> NativeWorkloadIdentity {
    NativeWorkloadIdentity {
        spiffe_id: w.spiffe_id,
        trust_domain: w.trust_domain,
        attested_at: w.attested_at.as_deref().and_then(|s| s.parse().ok()),
        attestor: w.attestor,
        selectors: w.selectors,
        client_id: w.client_id,
    }
}

fn wit_violation_to_native(v: PluginViolation) -> NativePluginViolation {
    NativePluginViolation {
        code: v.code,
        reason: v.reason,
        description: v.description,
        details: serde_json::from_str(&v.details).unwrap_or_default(),
        plugin_name: None,
        proto_error_code: v.proto_error_code,
    }
}

// ---------------------------------------------------------------------------
// WIT → Native: payload variant (for modified_payload in results)
// ---------------------------------------------------------------------------

/// Converts a WIT payload variant to a boxed native PluginPayload.
///
/// - `Payload::Message` → `Box<MessagePayload>` (fully typed, no JSON involved)
/// - `Payload::Custom`  → returns None (the executor receives the custom payload
///   JSON for downstream deserialization; the host doesn't know the concrete type)
fn wit_payload_variant_to_native(payload: Payload) -> Option<native_msg::MessagePayload> {
    match payload {
        Payload::Message(mp) => Some(wit_message_payload_to_native(mp)),
        Payload::Custom(_) => None,
    }
}

fn wit_message_payload_to_native(payload: MessagePayload) -> native_msg::MessagePayload {
    native_msg::MessagePayload {
        message: wit_message_to_native(payload.message),
    }
}

fn wit_message_to_native(msg: Message) -> native_msg::Message {
    native_msg::Message {
        schema_version: msg.schema_version,
        role: wit_role_to_native(msg.role),
        content: msg.content.into_iter().map(wit_content_part_to_native).collect(),
        channel: msg.channel.map(wit_channel_to_native),
    }
}

fn wit_role_to_native(role: Role) -> native_enums::Role {
    match role {
        Role::System => native_enums::Role::System,
        Role::Developer => native_enums::Role::Developer,
        Role::User => native_enums::Role::User,
        Role::Assistant => native_enums::Role::Assistant,
        Role::Tool => native_enums::Role::Tool,
    }
}

fn wit_channel_to_native(channel: Channel) -> native_enums::Channel {
    match channel {
        Channel::Analysis => native_enums::Channel::Analysis,
        Channel::Commentary => native_enums::Channel::Commentary,
        Channel::Final => native_enums::Channel::Final,
    }
}

fn wit_content_part_to_native(part: ContentPart) -> native_content::ContentPart {
    match part {
        ContentPart::Text(text) => native_content::ContentPart::Text { text },
        ContentPart::Thinking(text) => native_content::ContentPart::Thinking { text },
        ContentPart::ToolCall(tc) => native_content::ContentPart::ToolCall {
            content: wit_tool_call_to_native(tc),
        },
        ContentPart::ToolResult(tr) => native_content::ContentPart::ToolResult {
            content: wit_tool_result_to_native(tr),
        },
        ContentPart::CmfResource(r) => native_content::ContentPart::Resource {
            content: wit_resource_to_native(r),
        },
        ContentPart::ResourceRef(rr) => native_content::ContentPart::ResourceRef {
            content: wit_resource_ref_to_native(rr),
        },
        ContentPart::PromptRequest(pr) => native_content::ContentPart::PromptRequest {
            content: wit_prompt_request_to_native(pr),
        },
        ContentPart::PromptResult(pr) => native_content::ContentPart::PromptResult {
            content: wit_prompt_result_to_native(pr),
        },
        ContentPart::Image(img) => native_content::ContentPart::Image {
            content: native_content::ImageSource {
                source_type: img.source_type,
                data: img.data,
                media_type: img.media_type,
            },
        },
        ContentPart::Video(v) => native_content::ContentPart::Video {
            content: native_content::VideoSource {
                source_type: v.source_type,
                data: v.data,
                media_type: v.media_type,
                duration_ms: v.duration_ms,
            },
        },
        ContentPart::Audio(a) => native_content::ContentPart::Audio {
            content: native_content::AudioSource {
                source_type: a.source_type,
                data: a.data,
                media_type: a.media_type,
                duration_ms: a.duration_ms,
            },
        },
        ContentPart::Document(d) => native_content::ContentPart::Document {
            content: native_content::DocumentSource {
                source_type: d.source_type,
                data: d.data,
                media_type: d.media_type,
                title: d.title,
            },
        },
    }
}

fn wit_tool_call_to_native(tc: ToolCall) -> native_content::ToolCall {
    let arguments: HashMap<String, serde_json::Value> =
        serde_json::from_str(&tc.arguments).unwrap_or_default();
    native_content::ToolCall {
        tool_call_id: tc.tool_call_id,
        name: tc.name,
        arguments,
        namespace: tc.namespace,
    }
}

fn wit_tool_result_to_native(tr: ToolResult) -> native_content::ToolResult {
    let content: serde_json::Value =
        serde_json::from_str(&tr.content).unwrap_or(serde_json::Value::String(tr.content.clone()));
    native_content::ToolResult {
        tool_call_id: tr.tool_call_id,
        tool_name: tr.tool_name,
        content,
        is_error: tr.is_error,
    }
}

fn wit_resource_to_native(r: CmfResource) -> native_content::Resource {
    let annotations: HashMap<String, serde_json::Value> =
        serde_json::from_str(&r.annotations).unwrap_or_default();
    native_content::Resource {
        resource_request_id: r.resource_request_id,
        uri: r.uri,
        name: r.name,
        description: r.description,
        resource_type: wit_resource_type_to_native(r.resource_type),
        content: r.content,
        blob: r.blob,
        mime_type: r.mime_type,
        size_bytes: r.size_bytes,
        annotations,
        version: r.version,
    }
}

fn wit_resource_type_to_native(rt: ResourceType) -> native_enums::ResourceType {
    match rt {
        ResourceType::File => native_enums::ResourceType::File,
        ResourceType::Blob => native_enums::ResourceType::Blob,
        ResourceType::Uri => native_enums::ResourceType::Uri,
        ResourceType::Database => native_enums::ResourceType::Database,
        ResourceType::Api => native_enums::ResourceType::Api,
        ResourceType::Memory => native_enums::ResourceType::Memory,
        ResourceType::Artifact => native_enums::ResourceType::Artifact,
    }
}

fn wit_resource_ref_to_native(rr: ResourceReference) -> native_content::ResourceReference {
    native_content::ResourceReference {
        resource_request_id: rr.resource_request_id,
        uri: rr.uri,
        name: rr.name,
        resource_type: wit_resource_type_to_native(rr.resource_type),
        range_start: rr.range_start,
        range_end: rr.range_end,
        selector: rr.selector,
    }
}

fn wit_prompt_request_to_native(pr: PromptRequest) -> native_content::PromptRequest {
    let arguments: HashMap<String, serde_json::Value> =
        serde_json::from_str(&pr.arguments).unwrap_or_default();
    native_content::PromptRequest {
        prompt_request_id: pr.prompt_request_id,
        name: pr.name,
        arguments,
        server_id: pr.server_id,
    }
}

fn wit_prompt_result_to_native(pr: PromptResult) -> native_content::PromptResult {
    let messages: Vec<native_msg::Message> =
        serde_json::from_str(&pr.messages).unwrap_or_default();
    native_content::PromptResult {
        prompt_request_id: pr.prompt_request_id,
        prompt_name: pr.prompt_name,
        messages,
        content: pr.content,
        is_error: pr.is_error,
        error_message: pr.error_message,
    }
}
