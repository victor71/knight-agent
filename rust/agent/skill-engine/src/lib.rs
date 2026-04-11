//! Skill Engine
//!
//! Design Reference: docs/03-module-design/agent/skill-engine.md
//!
//! Manages skill registration, discovery, and execution.

pub mod manager;
pub mod types;

pub use types::{
    ExecutionPlan, ParameterType, Pipeline, PipelineStep, PlannedStep, SkillContext,
    SkillDefinition, SkillEngineError, SkillExecutionResult, SkillInfo, SkillParameter,
    SkillResult, SkillStep, StepType, Trigger, TriggerType,
};

pub use manager::SkillEngineImpl;
