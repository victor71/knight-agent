# Skill Engine Module

Design Reference: `docs/03-module-design/agent/skill-engine.md`

## 概述

Skill Engine 模块负责技能的注册、发现和执行管理：
- 技能注册与注销
- 技能分类管理
- 技能执行与结果追踪
- 执行计划创建
- 管道（Pipeline）执行

## 导入

```rust
use skill_engine::{
    SkillEngineImpl, SkillEngineError, SkillDefinition, SkillParameter, ParameterType,
    SkillStep, StepType, Trigger, TriggerType, SkillContext, SkillExecutionResult,
    SkillInfo, Pipeline, PipelineStep, ExecutionPlan, PlannedStep,
};
```

## 核心类型

### SkillEngineError
技能引擎错误枚举：
- `NotInitialized`: 未初始化
- `SkillNotFound(String)`: 技能未找到
- `ExecutionFailed(String)`: 技能执行失败
- `AlreadyRegistered(String)`: 技能已注册
- `InvalidDefinition(String)`: 无效的技能定义
- `PipelineError(String)`: 管道执行错误
- `TriggerError(String)`: 触发器错误

### SkillDefinition
技能定义：
- `id`: 技能唯一标识
- `name`: 技能名称
- `description`: 技能描述
- `category`: 技能分类（可选）
- `triggers`: 触发器列表
- `parameters`: 参数定义列表
- `steps`: 执行步骤列表
- `enabled`: 是否启用
- `version`: 版本号

### SkillParameter
技能参数定义：
- `name`: 参数名称
- `param_type`: 参数类型 (String, Integer, Float, Boolean, Object, Array)
- `description`: 参数描述
- `required`: 是否必需
- `default_value`: 默认值（可选）

### SkillStep
技能执行步骤：
- `id`: 步骤唯一标识
- `name`: 步骤名称
- `description`: 步骤描述
- `step_type`: 步骤类型 (Action, Skill, Agent, Condition)
- `tool`: 工具名称（可选）
- `skill_id`: 子技能ID（可选）
- `parameters`: 执行参数
- `condition`: 条件表达式（可选）

### Trigger
技能触发器：
- `id`: 触发器唯一标识
- `trigger_type`: 触发器类型 (Keyword, Event, Timer, FileChange)
- `pattern`: 匹配模式（可选）
- `event_type`: 事件类型（可选）

### SkillContext
技能执行上下文：
- `session_id`: 会话ID
- `variables`: 上下文变量
- `files`: 相关文件列表
- `metadata`: 元数据

### SkillExecutionResult
技能执行结果：
- `skill_id`: 技能ID
- `success`: 是否成功
- `output`: 输出结果（可选）
- `error`: 错误信息（可选）
- `steps_completed`: 已完成步骤列表
- `execution_time_ms`: 执行时间（毫秒）

## 对外接口

### 创建引擎

```rust
let engine = SkillEngineImpl::new();
```

### 注册技能

```rust
let skill = SkillDefinition::new("hello-skill", "Hello Skill", "Says hello")
    .with_category("greetings")
    .with_parameter(SkillParameter::new("name", ParameterType::String, "Name to greet"))
    .with_trigger(Trigger::new("t1", TriggerType::Keyword).with_pattern("hello"));

engine.register_skill(skill).await.unwrap();
```

### 获取技能

```rust
let skill = engine.get_skill("hello-skill").await.unwrap();
```

### 列出所有技能

```rust
let skills = engine.list_skills().await;
```

### 按分类列出技能

```rust
let greetings = engine.list_skills_by_category("greetings").await.unwrap();
```

### 列出所有分类

```rust
let categories = engine.list_categories().await;
```

### 更新技能

```rust
let updated = SkillDefinition::new("hello-skill", "Hello Skill Updated", "Updated description")
    .with_category("greetings");
engine.update_skill(updated).await.unwrap();
```

### 注销技能

```rust
engine.unregister_skill("hello-skill").await.unwrap();
```

### 执行技能

```rust
let context = SkillContext::new("session-1")
    .with_variable("name", serde_json::json!("World"));

let mut params = serde_json::Map::new();
params.insert("name".to_string(), serde_json::json!("World"));

let result = engine.execute_skill("hello-skill", &context, params).await.unwrap();
println!("Success: {}", result.success);
println!("Output: {:?}", result.output);
```

### 创建执行计划

```rust
let plan = engine.create_execution_plan("I want to say hello").await.unwrap();
println!("Plan confidence: {}", plan.confidence);
```

### 执行管道

```rust
let pipeline = Pipeline::new("hello-pipeline", "Hello Pipeline")
    .with_step(PipelineStep::new("hello-skill"));

let result = engine.execute_pipeline(&pipeline, &context).await.unwrap();
```

### 获取执行历史

```rust
let history = engine.get_execution_history(10).await;
```

### 清除执行历史

```rust
engine.clear_history().await;
```

### 检查技能是否存在

```rust
if engine.has_skill("hello-skill").await {
    println!("Skill exists!");
}
```

### 获取技能数量

```rust
let count = engine.skill_count().await;
```

## 完整示例

```rust
use skill_engine::{SkillEngineImpl, SkillDefinition, SkillParameter, ParameterType,
    SkillStep, StepType, Trigger, TriggerType, SkillContext};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = SkillEngineImpl::new();

    // Register a skill
    let skill = SkillDefinition::new("greet", "Greeting Skill", "Greets the user")
        .with_category("greetings")
        .with_trigger(Trigger::new("t1", TriggerType::Keyword).with_pattern("hello"))
        .with_parameter(SkillParameter::new("name", ParameterType::String, "Name to greet"));

    engine.register_skill(skill).await?;

    // List available skills
    let skills = engine.list_skills();
    println!("Available skills: {:?}", skills);

    // Execute the skill
    let context = SkillContext::new("session-1");
    let result = engine
        .execute_skill("greet", &context, serde_json::Map::new())
        .await?;

    println!("Execution result: {:?}", result);

    Ok(())
}
```

## 错误处理

所有操作都返回 `SkillResult<T>` 类型，使用 `?` 操作符进行错误传播：

```rust
match engine.register_skill(skill).await {
    Ok(_) => println!("Skill registered!"),
    Err(SkillEngineError::AlreadyRegistered(id)) => {
        println!("Skill {} already exists", id);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```
