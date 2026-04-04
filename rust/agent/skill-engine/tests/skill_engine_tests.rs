//! Skill Engine Tests
//!
//! Unit tests for the skill-engine module.

use skill_engine::{
    SkillEngineImpl, SkillEngineError, SkillDefinition, SkillParameter, ParameterType,
    SkillStep, StepType, Trigger, TriggerType, SkillContext, SkillExecutionResult,
    SkillInfo, Pipeline, PipelineStep, ExecutionPlan, PlannedStep,
};

// Manager tests

#[tokio::test]
async fn test_register_skill() {
    let engine = SkillEngineImpl::new();
    let skill = SkillDefinition::new("test-skill", "Test Skill", "A test skill")
        .with_category("testing");

    let result = engine.register_skill(skill).await;
    assert!(result.is_ok());
    assert_eq!(engine.skill_count().await, 1);
}

#[tokio::test]
async fn test_register_duplicate_skill() {
    let engine = SkillEngineImpl::new();
    let skill = SkillDefinition::new("test-skill", "Test Skill", "A test skill");

    engine.register_skill(skill.clone()).await.unwrap();
    let result = engine.register_skill(skill).await;
    assert!(matches!(result, Err(SkillEngineError::AlreadyRegistered(_))));
}

#[tokio::test]
async fn test_get_skill() {
    let engine = SkillEngineImpl::new();
    let skill = SkillDefinition::new("test-skill", "Test Skill", "A test skill");
    engine.register_skill(skill.clone()).await.unwrap();

    let retrieved = engine.get_skill("test-skill").await.unwrap();
    assert_eq!(retrieved.id, "test-skill");
}

#[tokio::test]
async fn test_get_skill_not_found() {
    let engine = SkillEngineImpl::new();
    let result = engine.get_skill("nonexistent").await;
    assert!(matches!(result, Err(SkillEngineError::SkillNotFound(_))));
}

#[tokio::test]
async fn test_list_skills() {
    let engine = SkillEngineImpl::new();
    let skill1 = SkillDefinition::new("skill-1", "Skill 1", "First skill");
    let skill2 = SkillDefinition::new("skill-2", "Skill 2", "Second skill");
    engine.register_skill(skill1).await.unwrap();
    engine.register_skill(skill2).await.unwrap();

    let skills = engine.list_skills().await;
    assert_eq!(skills.len(), 2);
}

#[tokio::test]
async fn test_list_skills_by_category() {
    let engine = SkillEngineImpl::new();
    let skill1 = SkillDefinition::new("skill-1", "Skill 1", "First skill")
        .with_category("testing");
    let skill2 = SkillDefinition::new("skill-2", "Skill 2", "Second skill")
        .with_category("other");
    engine.register_skill(skill1).await.unwrap();
    engine.register_skill(skill2).await.unwrap();

    let testing_skills = engine.list_skills_by_category("testing").await.unwrap();
    assert_eq!(testing_skills.len(), 1);
    assert_eq!(testing_skills[0].id, "skill-1");
}

#[tokio::test]
async fn test_list_categories() {
    let engine = SkillEngineImpl::new();
    let skill1 = SkillDefinition::new("skill-1", "Skill 1", "First skill")
        .with_category("testing");
    let skill2 = SkillDefinition::new("skill-2", "Skill 2", "Second skill")
        .with_category("production");
    engine.register_skill(skill1).await.unwrap();
    engine.register_skill(skill2).await.unwrap();

    let categories = engine.list_categories().await;
    assert_eq!(categories.len(), 2);
    assert!(categories.contains(&"testing".to_string()));
    assert!(categories.contains(&"production".to_string()));
}

#[tokio::test]
async fn test_unregister_skill() {
    let engine = SkillEngineImpl::new();
    let skill = SkillDefinition::new("test-skill", "Test Skill", "A test skill");
    engine.register_skill(skill).await.unwrap();

    let result = engine.unregister_skill("test-skill").await;
    assert!(result.is_ok());
    assert_eq!(engine.skill_count().await, 0);
}

#[tokio::test]
async fn test_unregister_nonexistent_skill() {
    let engine = SkillEngineImpl::new();
    let result = engine.unregister_skill("nonexistent").await;
    assert!(matches!(result, Err(SkillEngineError::SkillNotFound(_))));
}

#[tokio::test]
async fn test_update_skill() {
    let engine = SkillEngineImpl::new();
    let skill = SkillDefinition::new("test-skill", "Test Skill", "A test skill")
        .with_category("testing");
    engine.register_skill(skill).await.unwrap();

    let updated_skill = SkillDefinition::new("test-skill", "Updated Skill", "Updated description")
        .with_category("production");
    let result = engine.update_skill(updated_skill).await;
    assert!(result.is_ok());

    let retrieved = engine.get_skill("test-skill").await.unwrap();
    assert_eq!(retrieved.name, "Updated Skill");
}

#[tokio::test]
async fn test_execute_skill() {
    let engine = SkillEngineImpl::new();
    let skill = SkillDefinition::new("test-skill", "Test Skill", "A test skill");
    engine.register_skill(skill).await.unwrap();

    let context = SkillContext::new("session-1");
    let result = engine
        .execute_skill("test-skill", &context, serde_json::Map::new())
        .await;
    assert!(result.is_ok());
    assert!(result.unwrap().success);
}

#[tokio::test]
async fn test_execute_nonexistent_skill() {
    let engine = SkillEngineImpl::new();
    let context = SkillContext::new("session-1");
    let result = engine
        .execute_skill("nonexistent", &context, serde_json::Map::new())
        .await;
    assert!(matches!(result, Err(SkillEngineError::SkillNotFound(_))));
}

#[tokio::test]
async fn test_execute_skill_with_parameters() {
    let engine = SkillEngineImpl::new();
    let mut skill = SkillDefinition::new("test-skill", "Test Skill", "A test skill");
    skill.parameters.push(SkillParameter::new("name", ParameterType::String, "Name param"));
    engine.register_skill(skill).await.unwrap();

    let context = SkillContext::new("session-1");
    let mut params = serde_json::Map::new();
    params.insert("name".to_string(), serde_json::json!("test"));

    let result = engine.execute_skill("test-skill", &context, params).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_create_execution_plan() {
    let engine = SkillEngineImpl::new();
    let skill = SkillDefinition::new("test-skill", "Test Skill", "A test skill")
        .with_trigger(Trigger::new("t1", TriggerType::Keyword).with_pattern("test"));
    engine.register_skill(skill).await.unwrap();

    let plan = engine.create_execution_plan("This is a test task").await.unwrap();
    assert!(!plan.steps.is_empty());
}

#[tokio::test]
async fn test_execution_history() {
    let engine = SkillEngineImpl::new();
    let skill = SkillDefinition::new("test-skill", "Test Skill", "A test skill");
    engine.register_skill(skill).await.unwrap();

    let context = SkillContext::new("session-1");
    engine
        .execute_skill("test-skill", &context, serde_json::Map::new())
        .await
        .unwrap();

    let history = engine.get_execution_history(10).await;
    assert_eq!(history.len(), 1);
}

#[tokio::test]
async fn test_has_skill() {
    let engine = SkillEngineImpl::new();
    let skill = SkillDefinition::new("test-skill", "Test Skill", "A test skill");
    engine.register_skill(skill).await.unwrap();

    assert!(engine.has_skill("test-skill").await);
    assert!(!engine.has_skill("nonexistent").await);
}

// Type tests

#[test]
fn test_skill_definition_new() {
    let skill = SkillDefinition::new("test-skill", "Test Skill", "A test skill");
    assert_eq!(skill.id, "test-skill");
    assert_eq!(skill.name, "Test Skill");
    assert!(skill.enabled);
    assert_eq!(skill.version, "1.0.0");
}

#[test]
fn test_skill_definition_with_category() {
    let skill = SkillDefinition::new("test", "Test", "Test")
        .with_category("testing");
    assert_eq!(skill.category, Some("testing".to_string()));
}

#[test]
fn test_skill_definition_with_trigger() {
    let skill = SkillDefinition::new("test", "Test", "Test")
        .with_trigger(Trigger::new("t1", TriggerType::Keyword));
    assert_eq!(skill.triggers.len(), 1);
}

#[test]
fn test_skill_definition_with_parameter() {
    let skill = SkillDefinition::new("test", "Test", "Test")
        .with_parameter(SkillParameter::new("param1", ParameterType::String, "A parameter"));
    assert_eq!(skill.parameters.len(), 1);
}

#[test]
fn test_skill_definition_with_step() {
    let skill = SkillDefinition::new("test", "Test", "Test")
        .with_step(SkillStep::new("step1", "Step 1", "First step"));
    assert_eq!(skill.steps.len(), 1);
}

#[test]
fn test_skill_parameter_new() {
    let param = SkillParameter::new("name", ParameterType::String, "A name parameter");
    assert_eq!(param.name, "name");
    assert_eq!(param.param_type, ParameterType::String);
    assert!(param.required);
    assert!(param.default_value.is_none());
}

#[test]
fn test_skill_parameter_optional() {
    let param = SkillParameter::new("opt", ParameterType::Integer, "Optional")
        .optional();
    assert!(!param.required);
}

#[test]
fn test_skill_parameter_with_default() {
    let param = SkillParameter::new("with_default", ParameterType::String, "With default")
        .with_default(serde_json::json!("default_value"));
    assert!(!param.required);
    assert!(param.default_value.is_some());
}

#[test]
fn test_parameter_type_default() {
    let param_type = ParameterType::default();
    assert_eq!(param_type, ParameterType::String);
}

#[test]
fn test_skill_step_new() {
    let step = SkillStep::new("step1", "Test Step", "A test step");
    assert_eq!(step.id, "step1");
    assert_eq!(step.name, "Test Step");
    assert_eq!(step.step_type, StepType::Action);
    assert!(step.tool.is_none());
    assert!(step.skill_id.is_none());
}

#[test]
fn test_skill_step_with_tool() {
    let step = SkillStep::new("step1", "Test Step", "A test step")
        .with_tool("read_file");
    assert_eq!(step.tool, Some("read_file".to_string()));
}

#[test]
fn test_skill_step_with_skill() {
    let step = SkillStep::new("step1", "Test Step", "A test step")
        .with_skill("other-skill");
    assert_eq!(step.skill_id, Some("other-skill".to_string()));
    assert_eq!(step.step_type, StepType::Skill);
}

#[test]
fn test_step_type_default() {
    let step_type = StepType::default();
    assert_eq!(step_type, StepType::Action);
}

#[test]
fn test_trigger_new() {
    let trigger = Trigger::new("t1", TriggerType::Keyword);
    assert_eq!(trigger.id, "t1");
    assert_eq!(trigger.trigger_type, TriggerType::Keyword);
    assert!(trigger.pattern.is_none());
    assert!(trigger.event_type.is_none());
}

#[test]
fn test_trigger_with_pattern() {
    let trigger = Trigger::new("t1", TriggerType::Keyword)
        .with_pattern("test");
    assert_eq!(trigger.pattern, Some("test".to_string()));
}

#[test]
fn test_trigger_with_event_type() {
    let trigger = Trigger::new("t1", TriggerType::Event)
        .with_event_type("file_changed");
    assert_eq!(trigger.event_type, Some("file_changed".to_string()));
}

#[test]
fn test_trigger_type_default() {
    let trigger_type = TriggerType::default();
    assert_eq!(trigger_type, TriggerType::Keyword);
}

#[test]
fn test_skill_context_new() {
    let ctx = SkillContext::new("session-1");
    assert_eq!(ctx.session_id, "session-1");
    assert!(ctx.variables.is_empty());
    assert!(ctx.files.is_empty());
}

#[test]
fn test_skill_context_with_variable() {
    let ctx = SkillContext::new("session-1")
        .with_variable("name", serde_json::json!("value"));
    assert!(ctx.variables.contains_key("name"));
}

#[test]
fn test_skill_context_with_file() {
    let ctx = SkillContext::new("session-1")
        .with_file("/path/to/file.txt");
    assert_eq!(ctx.files.len(), 1);
}

#[test]
fn test_execution_result_success() {
    let result = SkillExecutionResult::success(
        "skill-1",
        serde_json::json!({"result": "ok"}),
        100,
    );
    assert!(result.success);
    assert!(result.output.is_some());
    assert!(result.error.is_none());
    assert_eq!(result.execution_time_ms, 100);
}

#[test]
fn test_execution_result_failure() {
    let result = SkillExecutionResult::failure("skill-1", "Error occurred", 50);
    assert!(!result.success);
    assert!(result.output.is_none());
    assert_eq!(result.error, Some("Error occurred".to_string()));
    assert_eq!(result.execution_time_ms, 50);
}

#[test]
fn test_execution_result_with_steps() {
    let result = SkillExecutionResult::success("skill-1", serde_json::json!({}), 100)
        .with_steps_completed(vec!["step1".to_string(), "step2".to_string()]);
    assert_eq!(result.steps_completed.len(), 2);
}

#[test]
fn test_skill_info_from_definition() {
    let skill = SkillDefinition::new("test-skill", "Test Skill", "A test skill")
        .with_category("testing");
    let info = SkillInfo::from(&skill);
    assert_eq!(info.id, "test-skill");
    assert_eq!(info.name, "Test Skill");
    assert_eq!(info.category, Some("testing".to_string()));
    assert!(info.enabled);
}

#[test]
fn test_pipeline_new() {
    let pipeline = Pipeline::new("pipeline-1", "Test Pipeline");
    assert_eq!(pipeline.id, "pipeline-1");
    assert_eq!(pipeline.name, "Test Pipeline");
    assert!(pipeline.steps.is_empty());
}

#[test]
fn test_pipeline_with_step() {
    let pipeline = Pipeline::new("pipeline-1", "Test Pipeline")
        .with_step(PipelineStep::new("skill-1"));
    assert_eq!(pipeline.steps.len(), 1);
}

#[test]
fn test_pipeline_step_new() {
    let step = PipelineStep::new("skill-1");
    assert_eq!(step.skill_id, "skill-1");
    assert!(step.parameters.is_empty());
    assert!(step.condition.is_none());
}

#[test]
fn test_execution_plan_new() {
    let plan = ExecutionPlan::new();
    assert!(plan.steps.is_empty());
    assert_eq!(plan.confidence, 1.0);
}

#[test]
fn test_planned_step() {
    let step = PlannedStep {
        sequence: 0,
        skill_id: "skill-1".to_string(),
        parameters: serde_json::Map::new(),
    };
    assert_eq!(step.sequence, 0);
    assert_eq!(step.skill_id, "skill-1");
}
