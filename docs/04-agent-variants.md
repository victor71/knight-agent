# Agent 变体支持设计文档

## 1. 概述

### 1.1 什么是 Agent 变体

Agent 变体（Variant）是指同一个 Agent 的不同配置版本，**并行存在**，用于不同使用场景。

```
code-reviewer Agent
├── 默认变体      - 完整审查，使用 Sonnet
├── quick 变体    - 快速检查，使用 Haiku
├── security 变体 - 安全专项，深度分析
└── fixer 变体    - 不仅审查，还直接修复
```

### 1.2 变体 vs 版本

| 维度 | 变体 (Variant) | 版本 (Version) |
|------|----------------|----------------|
| **目的** | 不同场景不同配置 | 功能升级演进 |
| **存在方式** | 并行共存 | 先后替换 |
| **命名示例** | `quick`, `full`, `security` | `1.0.0`, `1.1.0`, `2.0.0` |
| **切换方式** | `agent:variant` | `--version x.y.z` |
| **兼容性** | 可能差异很大 | 向后兼容 |
| **适用阶段** | 即时需要 | 成熟后 |
| **优先级** | **P1** | P3 |

### 1.3 使用场景

**场景 1: 快速 vs 深度**
```bash
# 开发时快速检查
knight ask reviewer:quick "看看这段"

# 提交前深度审查
knight ask reviewer:full "全面审查"
```

**场景 2: 不同模型**
```bash
# 简单任务用便宜的模型
knight ask coder:lite "写个排序函数"

# 复杂任务用强大的模型
knight ask coder:pro "重构整个模块"
```

**场景 3: 专项任务**
```bash
knight ask reviewer:security "检查安全漏洞"
knight ask reviewer:performance "分析性能瓶颈"
knight ask reviewer:style "检查代码风格"
```

---

## 2. 变体定义格式

### 2.1 基础结构

```
agents/{agent-name}/
├── AGENT.md              # 主定义（默认变体）
├── AGENT.{variant}.md    # 变体定义
└── _variants/            # 或者放在子目录
    ├── quick.md
    ├── full.md
    └── security.md
```

### 2.2 继承机制

变体可以继承主定义，只覆盖需要修改的部分：

```markdown
---
extends: AGENT.md         # 继承基础定义
variant: quick            # 声明变体名称
---

## Role
快速代码审查助手

## Model
- provider: anthropic
- model: claude-haiku      # 覆盖：使用更快模型
- temperature: 0.1         # 覆盖：更确定性

## Instructions (覆盖)
只检查：
1. 语法错误
2. 常见反模式
3. 命名规范

跳过深度分析和性能评估。
```

### 2.3 完整示例

**主定义** (`AGENT.md`):
```markdown
---
id: "code-reviewer"
name: "Code Reviewer"
version: "1.0.0"
---

# Agent: Code Reviewer

## Role
专业的代码审查助手

## Model
- provider: anthropic
- model: claude-sonnet-4-6
- temperature: 0.3
- max_tokens: 8192

## Instructions
检查代码的：
1. 安全性
2. 性能
3. 可读性
4. 最佳实践

## Capabilities
- read
- grep
- bash (lint)
```

**快速变体** (`AGENT.quick.md`):
```markdown
---
extends: AGENT.md
variant: quick
---

## Role
快速代码检查

## Model
- model: claude-haiku
- max_tokens: 4096

## Instructions
只检查：
1. 明显错误
2. 命名规范
3. 简单反模式
```

**安全变体** (`AGENT.security.md`):
```markdown
---
extends: AGENT.md
variant: security
---

## Role
安全专项审查

## Model
- temperature: 0.1        # 更确定性
- max_tokens: 16384       # 更长输出

## Instructions
专注于安全检查：
1. SQL 注入
2. XSS 漏洞
3. 敏感信息泄露
4. 认证授权问题

## Skills
- security-scan
- secret-detection
```

**修复变体** (`AGENT.fixer.md`):
```markdown
---
extends: AGENT.md
variant: fixer
---

## Role
代码审查并修复

## Instructions
不仅发现问题，还要：
1. 提供修复代码
2. 直接应用修复（经确认）
3. 运行测试验证

## Capabilities
- read
- write
- edit          # 新增：编辑能力
- bash (test)
```

---

## 3. 变体解析机制

### 3.1 解析流程

```rust
pub struct AgentVariantLoader {
    base_dir: PathBuf,
}

impl AgentVariantLoader {
    /// 加载 Agent 定义（支持变体）
    pub fn load(&self, agent_id: &str, variant: Option<&str>)
        -> Result<AgentDefinition>
    {
        let base_path = self.base_dir.join(agent_id);

        // 1. 确定要加载的文件
        let file_path = match variant {
            Some(v) => base_dir.join(format!("AGENT.{}.md", v)),
            None => base_path.join("AGENT.md"),
        };

        // 2. 解析目标文件
        let mut definition = AgentDefinition::from_file(&file_path)?;

        // 3. 处理继承
        if let Some extends) = &definition.extends {
            let base_path = base_path.join(extends);
            let base_def = AgentDefinition::from_file(&base_path)?;
            definition = definition.merge(base_def)?;
        }

        // 4. 设置变体信息
        definition.variant = variant.map(|s| s.to_string());

        Ok(definition)
    }
}

impl AgentDefinition {
    /// 合并定义（变体覆盖基类）
    fn merge(mut self, base: AgentDefinition) -> Result<Self> {
        // 保留变体特有的值，其他从 base 继承
        self.id = self.id.unwrap_or(base.id);
        self.name = self.name.unwrap_or(base.name);

        // Model: 完全覆盖
        if self.model.is_none() {
            self.model = base.model;
        }

        // Instructions: 追加或覆盖
        if self.instructions.is_empty() {
            self.instructions = base.instructions;
        }

        // Capabilities: 合并
        self.capabilities.extend(base.capabilities);

        Ok(self)
    }
}
```

### 3.2 变体发现

```rust
impl AgentVariantLoader {
    /// 列出所有可用变体
    pub fn list_variants(&self, agent_id: &str) -> Result<Vec<String>> {
        let agent_dir = self.base_dir.join(agent_id);
        let mut variants = Vec::new();

        // 扫描 AGENT.*.md 文件
        for entry in std::fs::read_dir(agent_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if name_str.starts_with("AGENT.") && name_str.ends_with(".md") {
                // 提取变体名: AGENT.quick.md -> quick
                let variant = name_str
                    .strip_prefix("AGENT.")
                    .and_then(|s| s.strip_suffix(".md"))
                    .map(|s| s.to_string());

                if let Some(v) = variant {
                    variants.push(v);
                }
            }
        }

        variants.sort();
        Ok(variants)
    }

    /// 获取变体信息
    pub fn get_variant_info(&self, agent_id: &str, variant: &str)
        -> Result<VariantInfo>
    {
        let path = self.base_dir
            .join(agent_id)
            .join(format!("AGENT.{}.md", variant));

        let content = std::fs::read_to_string(&path)?;

        // 解析 frontmatter
        let info: VariantInfo = parse_frontmatter(&content)?;

        Ok(info)
    }
}

pub struct VariantInfo {
    pub variant: String,
    pub description: Option<String>,
    pub model: Option<String>,
    pub extends: Option<String>,
}
```

---

## 4. CLI 交互

### 4.1 基本用法

```bash
# 列出所有 Agent 及其变体
knight agent list
# 输出:
# code-reviewer (default, quick, security, fixer)
# coder (default, lite, pro)

# 使用默认变体
knight ask code-reviewer "审查这段代码"

# 指定变体（语法 1: 冒号）
knight ask code-reviewer:quick "快速检查"

# 指定变体（语法 2: 空格）
knight ask code-reviewer --variant quick "快速检查"

# 查看变体信息
knight agent info code-reviewer --variant quick
```

### 4.2 交互模式

```bash
# 启动特定变体
knight chat code-reviewer:quick

# 运行中切换变体
» quick "检查这个"      # 使用 quick 变体
» switch full           # 切换到 full 变体
» "深度审查"            # 使用 full 变体
```

### 4.3 工作流中使用

```yaml
# workflows/pr-check.yaml
name: "PR 检查流程"

steps:
  # 快速检查
  - name: quick_check
    agent: code-reviewer:quick
    inputs:
      files: "{{ changed_files }}"

  # 如果通过，安全检查
  - name: security_check
    agent: code-reviewer:security
    run_if: quick_check.status == "pass"
    inputs:
      files: "{{ changed_files }}"

  # 最终报告
  - name: full_review
    agent: code-reviewer:full
    run_if: security_check.status == "pass"
```

---

## 5. 最佳实践

### 5.1 命名规范

| 变体名 | 用途 | 示例 |
|--------|------|------|
| `quick` | 快速检查 | 使用 Haiku，只检查明显问题 |
| `full` | 完整检查 | 使用 Sonnet，全面分析 |
| `lite` | 轻量版 | 简化能力，节省成本 |
| `pro` | 专业版 | 更多能力，更详细输出 |
| `{专项}` | 专项任务 | `security`, `performance`, `style` |
| `fixer` | 修复型 | 不仅分析，还修复 |

### 5.2 变体设计原则

1. **明确用途** - 每个变体有清晰的使用场景
2. **最小差异** - 只覆盖必要的配置
3. **继承优先** - 复用主定义，减少重复
4. **文档完整** - 说明何时使用哪个变体

### 5.3 何时创建变体

**适合创建变体**:
- 不同复杂度任务（quick vs full）
- 不同模型选择（lite vs pro）
- 不同专业领域（security vs performance）
- 不同行为模式（reviewer vs fixer）

**不适合创建变体**:
- 简单的参数调整 → 用配置文件
- 临时修改 → 用命令行参数
- 版本升级 → 用版本管理

---

## 6. 实现优先级

| 阶段 | 功能 |
|------|------|
| **P1 - MVP** |
| ✓ 基础变体定义 | 文件命名约定 `AGENT.{variant}.md` |
| ✓ 变体加载 | 解析 `extends` 字段并合并 |
| ✓ CLI 语法 | `agent:variant` 语法支持 |
| **P1.1** |
| ✓ 变体发现 | `list --variants` 命令 |
| ✓ 变体信息 | 查看变体详情 |
| **P2** |
| ○ 变体验证 | 确保变体定义合法 |
| ○ 变体测试 | 比较不同变体输出 |

---

## 7. 与其他功能的关系

### 7.1 与版本管理

```
变体: 并行存在，不同场景
版本: 先后演进，功能升级

可以同时存在：
code-reviewer v1.0.0
├── quick 变体
├── full 变体
└── security 变体

code-reviewer v2.0.0
├── quick 变体
├── full 变体
└── security 变体
```

### 7.2 与继承

```markdown
# 继承 + 变体 组合

# AGENT.md (基类)
└── AGENT.quick.md (继承基类)
    └── AGENT.quick.minimal.md (继承 quick 变体)
```

### 7.3 与配置

```yaml
# 项目配置中指定变体
project:
  agents:
    reviewer:
      agent: code-reviewer
      variant: quick              # 默认用 quick
      fallback_variant: full      # quick 失败时用 full
```

---

## 8. 示例场景

### 场景 1: CI/CD 分层检查

```yaml
# .github/workflows/pr-check.yml
name: PR Check

on: [pull_request]

jobs:
  quick_check:
    - agent: code-reviewer:quick
      # 快速反馈，5 分钟内完成

  full_check:
    needs: quick_check
    if: quick_check == pass
    - agent: code-reviewer:full
      # 全面审查，合并前执行
```

### 场景 2: 本地开发

```bash
# 写代码时快速检查
knight watch --agent reviewer:quick

# 提交前全面检查
knight ask reviewer:full "审查我的改动"
```

### 场景 3: 成本优化

```yaml
# 根据任务复杂度自动选择变体
rules:
  - if: task.lines < 100
    use_variant: quick
  - if: task.lines < 500
    use_variant: full
  - if: task.has_security_keyword
    use_variant: security
```

---

## 9. 总结

| 特性 | 价值 |
|------|------|
| **灵活** | 不同场景不同配置 |
| **简单** | 文件命名约定，无需复杂系统 |
| **实用** | 立即可用，解决实际问题 |
| **低成本** | 实现简单，P1 优先级 |

**推荐路径**: 先实现变体（P1），满足实际需求；版本管理留到后期（P3），当真正需要时再实现。
