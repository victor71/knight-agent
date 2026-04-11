//! Orchestrator Tests
//!
//! Unit tests for the orchestrator module.

use orchestrator::{
    AgentFilter, AgentInfo, AgentMessage, AgentStatus, CollaborationMode, OrchestratorError,
    OrchestratorImpl, TaskRequirements, TopicMessage,
};

#[tokio::test]
async fn test_register_agent() {
    let orch = OrchestratorImpl::new();
    let agent = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );

    let result = orch.register_agent(agent).await;
    assert!(result.is_ok());
    assert_eq!(orch.agent_count().await, 1);
}

#[tokio::test]
async fn test_register_duplicate_agent() {
    let orch = OrchestratorImpl::new();
    let agent = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );

    orch.register_agent(agent.clone()).await.unwrap();
    let result = orch.register_agent(agent).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_unregister_agent() {
    let orch = OrchestratorImpl::new();
    let agent = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );

    orch.register_agent(agent).await.unwrap();
    let result = orch.unregister_agent("agent-1").await;
    assert!(result.is_ok());
    assert_eq!(orch.agent_count().await, 0);
}

#[tokio::test]
async fn test_unregister_nonexistent_agent() {
    let orch = OrchestratorImpl::new();
    let result = orch.unregister_agent("nonexistent").await;
    assert!(matches!(result, Err(OrchestratorError::AgentNotFound(_))));
}

#[tokio::test]
async fn test_list_agents() {
    let orch = OrchestratorImpl::new();
    let agent1 = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );
    let agent2 = AgentInfo::new(
        "agent-2".to_string(),
        "Agent Two".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );

    orch.register_agent(agent1).await.unwrap();
    orch.register_agent(agent2).await.unwrap();

    let agents = orch.list_agents(None).await;
    assert_eq!(agents.len(), 2);
}

#[tokio::test]
async fn test_list_agents_filter() {
    let orch = OrchestratorImpl::new();
    let agent1 = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    )
    .with_variant("developer");

    orch.register_agent(agent1).await.unwrap();

    let filter = AgentFilter {
        variant: Some("developer".to_string()),
        ..Default::default()
    };
    let agents = orch.list_agents(Some(filter)).await;
    assert_eq!(agents.len(), 1);
}

#[tokio::test]
async fn test_get_agent_info() {
    let orch = OrchestratorImpl::new();
    let agent = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );

    orch.register_agent(agent).await.unwrap();

    let info = orch.get_agent_info("agent-1").await.unwrap();
    assert_eq!(info.name, "Agent One");
}

#[tokio::test]
async fn test_get_agent_info_not_found() {
    let orch = OrchestratorImpl::new();
    let result = orch.get_agent_info("nonexistent").await;
    assert!(matches!(result, Err(OrchestratorError::AgentNotFound(_))));
}

#[tokio::test]
async fn test_update_agent_status() {
    let orch = OrchestratorImpl::new();
    let agent = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );

    orch.register_agent(agent).await.unwrap();
    orch.update_agent_status("agent-1", AgentStatus::Busy)
        .await
        .unwrap();

    let info = orch.get_agent_info("agent-1").await.unwrap();
    assert_eq!(info.status, AgentStatus::Busy);
}

#[tokio::test]
async fn test_update_agent_task() {
    let orch = OrchestratorImpl::new();
    let agent = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );

    orch.register_agent(agent).await.unwrap();
    orch.update_agent_task("agent-1", Some("task-1".to_string()))
        .await
        .unwrap();

    let info = orch.get_agent_info("agent-1").await.unwrap();
    assert_eq!(info.current_task, Some("task-1".to_string()));
}

#[tokio::test]
async fn test_allocate_agent() {
    let orch = OrchestratorImpl::new();
    let agent = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );

    orch.register_agent(agent).await.unwrap();

    let requirements = TaskRequirements::default();
    let agent_id = orch.allocate_agent(&requirements).await.unwrap();
    assert_eq!(agent_id, "agent-1");

    let info = orch.get_agent_info("agent-1").await.unwrap();
    assert_eq!(info.status, AgentStatus::Busy);
}

#[tokio::test]
async fn test_allocate_no_available_agent() {
    let orch = OrchestratorImpl::new();
    // No agents registered

    let requirements = TaskRequirements::default();
    let result = orch.allocate_agent(&requirements).await;
    assert!(matches!(
        result,
        Err(OrchestratorError::AgentNotAvailable(_))
    ));
}

#[tokio::test]
async fn test_send_message() {
    let orch = OrchestratorImpl::new();
    let agent = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );

    orch.register_agent(agent).await.unwrap();

    let msg = AgentMessage::new("system", "agent-1", serde_json::json!("hello"));
    let result = orch.send_message("agent-1", msg).await;
    assert!(result.is_ok());

    let messages = orch.get_messages("agent-1").await;
    assert_eq!(messages.len(), 1);
}

#[tokio::test]
async fn test_broadcast() {
    let orch = OrchestratorImpl::new();
    let agent1 = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );
    let agent2 = AgentInfo::new(
        "agent-2".to_string(),
        "Agent Two".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );

    orch.register_agent(agent1).await.unwrap();
    orch.register_agent(agent2).await.unwrap();

    let msg = AgentMessage::new("system", "broadcast", serde_json::json!("hello all"));
    let results = orch
        .broadcast(&["agent-1".to_string(), "agent-2".to_string()], msg)
        .await;

    assert_eq!(results.len(), 2);
    assert!(results[0].success);
    assert!(results[1].success);
}

#[tokio::test]
async fn test_subscribe_and_publish() {
    let orch = OrchestratorImpl::new();
    let agent = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );

    orch.register_agent(agent).await.unwrap();
    orch.subscribe("agent-1", "code-changes").await.unwrap();

    let msg = TopicMessage::new("code-changes", "agent-2", serde_json::json!("file changed"));
    let count = orch.publish("code-changes", msg).await.unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_unsubscribe() {
    let orch = OrchestratorImpl::new();
    let agent = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );

    orch.register_agent(agent).await.unwrap();
    orch.subscribe("agent-1", "code-changes").await.unwrap();
    orch.unsubscribe("agent-1", "code-changes").await.unwrap();

    // Publishing should now fail since no subscribers
    let msg = TopicMessage::new("code-changes", "agent-2", serde_json::json!("file changed"));
    let result = orch.publish("code-changes", msg).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_collaboration() {
    let orch = OrchestratorImpl::new();
    let collab_id = orch
        .create_collaboration(
            "test-collab",
            vec!["agent-1".to_string(), "agent-2".to_string()],
            CollaborationMode::MasterWorker,
        )
        .await
        .unwrap();

    let collab = orch.get_collaboration(&collab_id).await.unwrap();
    assert_eq!(collab.name, "test-collab");
    assert_eq!(collab.agents.len(), 2);
    assert!(collab.master.is_some());
}

#[tokio::test]
async fn test_create_pipeline_collaboration() {
    let orch = OrchestratorImpl::new();
    let collab_id = orch
        .create_collaboration(
            "pipeline-collab",
            vec![
                "agent-1".to_string(),
                "agent-2".to_string(),
                "agent-3".to_string(),
            ],
            CollaborationMode::Pipeline,
        )
        .await
        .unwrap();

    let collab = orch.get_collaboration(&collab_id).await.unwrap();
    assert_eq!(collab.mode, CollaborationMode::Pipeline);
    assert!(collab.pipeline.len() == 3);
}

#[tokio::test]
async fn test_dissolve_collaboration() {
    let orch = OrchestratorImpl::new();
    let collab_id = orch
        .create_collaboration(
            "test-collab",
            vec!["agent-1".to_string()],
            CollaborationMode::MasterWorker,
        )
        .await
        .unwrap();

    orch.dissolve_collaboration(&collab_id).await.unwrap();

    let result = orch.get_collaboration(&collab_id).await;
    assert!(matches!(
        result,
        Err(OrchestratorError::CollaborationNotFound(_))
    ));
}

#[tokio::test]
async fn test_get_resource_usage() {
    let orch = OrchestratorImpl::new();
    let agent = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );

    orch.register_agent(agent).await.unwrap();

    let usage = orch.get_resource_usage().await;
    assert_eq!(usage.total_agents, 1);
    assert_eq!(usage.active_agents, 0);
}

#[tokio::test]
async fn test_record_task_completion() {
    let orch = OrchestratorImpl::new();
    let agent = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );

    orch.register_agent(agent).await.unwrap();
    orch.update_agent_status("agent-1", AgentStatus::Busy)
        .await
        .unwrap();
    orch.update_agent_task("agent-1", Some("task-1".to_string()))
        .await
        .unwrap();

    orch.record_task_completion("agent-1", 1000).await.unwrap();

    let info = orch.get_agent_info("agent-1").await.unwrap();
    assert_eq!(info.status, AgentStatus::Idle);
    assert!(info.current_task.is_none());
    assert_eq!(info.statistics.tasks_completed, 1);
}

#[tokio::test]
async fn test_record_task_failure() {
    let orch = OrchestratorImpl::new();
    let agent = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );

    orch.register_agent(agent).await.unwrap();
    orch.update_agent_status("agent-1", AgentStatus::Busy)
        .await
        .unwrap();

    orch.record_task_failure("agent-1").await.unwrap();

    let info = orch.get_agent_info("agent-1").await.unwrap();
    assert_eq!(info.status, AgentStatus::Error);
    assert_eq!(info.statistics.tasks_failed, 1);
}

#[tokio::test]
async fn test_has_agent() {
    let orch = OrchestratorImpl::new();
    let agent = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );

    orch.register_agent(agent).await.unwrap();

    assert!(orch.has_agent("agent-1").await);
    assert!(!orch.has_agent("nonexistent").await);
}

#[tokio::test]
async fn test_get_messages() {
    let orch = OrchestratorImpl::new();
    let agent = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );

    orch.register_agent(agent).await.unwrap();

    let msg1 = AgentMessage::new("system", "agent-1", serde_json::json!("hello"));
    let msg2 = AgentMessage::new("system", "agent-1", serde_json::json!("world"));
    orch.send_message("agent-1", msg1).await.unwrap();
    orch.send_message("agent-1", msg2).await.unwrap();

    let messages = orch.get_messages("agent-1").await;
    assert_eq!(messages.len(), 2);
}

#[tokio::test]
async fn test_agent_count() {
    let orch = OrchestratorImpl::new();
    assert_eq!(orch.agent_count().await, 0);

    let agent = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );
    orch.register_agent(agent).await.unwrap();

    assert_eq!(orch.agent_count().await, 1);
}

// Type tests

#[test]
fn test_agent_info_new() {
    let info = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    );
    assert_eq!(info.id, "agent-1");
    assert_eq!(info.status, AgentStatus::Idle);
}

#[test]
fn test_agent_info_with_variant() {
    let info = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    )
    .with_variant("developer");
    assert_eq!(info.variant, Some("developer".to_string()));
}

#[test]
fn test_agent_info_with_capabilities() {
    let info = AgentInfo::new(
        "agent-1".to_string(),
        "Agent One".to_string(),
        "claude".to_string(),
        "session-1".to_string(),
    )
    .with_capabilities(vec!["coding".to_string(), "review".to_string()]);
    assert_eq!(info.capabilities.len(), 2);
}

#[test]
fn test_collaboration_new() {
    let collab = orchestrator::Collaboration::new(
        "collab-1",
        "Test Collaboration",
        vec!["agent-1".to_string(), "agent-2".to_string()],
        CollaborationMode::MasterWorker,
    );
    assert_eq!(collab.id, "collab-1");
    assert_eq!(collab.agents.len(), 2);
}

#[test]
fn test_collaboration_with_master() {
    let collab = orchestrator::Collaboration::new(
        "collab-1",
        "Test Collaboration",
        vec!["agent-1".to_string(), "agent-2".to_string()],
        CollaborationMode::MasterWorker,
    )
    .with_master("agent-1");
    assert_eq!(collab.master, Some("agent-1".to_string()));
}

#[test]
fn test_collaboration_with_pipeline() {
    let collab = orchestrator::Collaboration::new(
        "collab-1",
        "Test Collaboration",
        vec!["agent-1".to_string(), "agent-2".to_string()],
        CollaborationMode::Pipeline,
    )
    .with_pipeline(vec!["agent-1".to_string(), "agent-2".to_string()]);
    assert_eq!(collab.pipeline.len(), 2);
}

#[test]
fn test_task_requirements_default() {
    let reqs = TaskRequirements::default();
    assert!(reqs.create_if_missing);
}

#[test]
fn test_send_result_success() {
    let result = orchestrator::SendResult::success("agent-1");
    assert!(result.success);
    assert!(result.error.is_none());
}

#[test]
fn test_send_result_failure() {
    let result = orchestrator::SendResult::failure("agent-1", "Agent not responding");
    assert!(!result.success);
    assert_eq!(result.error, Some("Agent not responding".to_string()));
}

#[test]
fn test_topic_subscription_new() {
    let sub = orchestrator::TopicSubscription::new("agent-1", "code-changes");
    assert_eq!(sub.agent_id, "agent-1");
    assert_eq!(sub.topic, "code-changes");
}

#[test]
fn test_agent_message_new() {
    let msg = AgentMessage::new("agent-1", "agent-2", serde_json::json!({"text": "hello"}));
    assert_eq!(msg.from, "agent-1");
    assert_eq!(msg.to, "agent-2");
}

#[test]
fn test_topic_message_new() {
    let msg = orchestrator::TopicMessage::new(
        "code-changes",
        "agent-1",
        serde_json::json!({"file": "main.rs"}),
    );
    assert_eq!(msg.topic, "code-changes");
    assert_eq!(msg.from, "agent-1");
}
