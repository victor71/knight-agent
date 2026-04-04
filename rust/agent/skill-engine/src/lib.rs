//! Skill Engine
//!
//! Design Reference: docs/03-module-design/agent/skill-engine.md
//!
//! Manages skill registration, discovery, and execution.

pub mod types;
pub mod manager;

pub use types::{
    SkillEngineError, SkillResult, SkillDefinition, SkillParameter, ParameterType,
    SkillStep, StepType, Trigger, TriggerType, SkillContext, SkillExecutionResult,
    SkillInfo, Pipeline, PipelineStep, ExecutionPlan, PlannedStep,
};

pub use manager::SkillEngineImpl;
