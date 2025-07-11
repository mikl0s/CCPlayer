# Unified PRP Generator and Executor

## Feature file: $ARGUMENTS

**MANDATORY DYNAMIC SUB-AGENT EXECUTION** - Generate complete PRP and execute feature implementation using Claude Code's unlimited sub-agent spawning capabilities.

## Phase 1: Comprehensive Research WITH DYNAMIC SUB-AGENT EXECUTION

**CRITICAL: Execute thorough research phases using Claude Code's unlimited sub-agent spawning with dependency awareness**

Read the feature file first to understand what needs to be created, how the examples provided help, and any other considerations.

The Claude Code agent only gets the context you are appending to the internal research findings and training data. Assume the Claude Code agent has access to the codebase and the same knowledge cutoff as you, so its important that your research findings are included or referenced. The Agent has Websearch capabilities, so pass urls to documentation and examples.

### Initial Research Wave (No Dependencies - Spawn Immediately)

1. **Sub-Agent Research 1: Codebase Analysis**
   - Search for similar features/patterns in the codebase
   - Identify files to reference in implementation
   - Note existing conventions to follow
   - Check test patterns for validation approach

2. **Sub-Agent Research 2: External Research**
   - Search for similar features/patterns online
   - Library documentation (include specific URLs)
   - Implementation examples (GitHub/StackOverflow/blogs)
   - Best practices and common pitfalls

3. **Sub-Agent Research 3: User Clarification** (if needed)
   - Specific patterns to mirror and where to find them?
   - Integration requirements and where to find them?

### Dependency-Triggered Research (Spawn as initial research completes)
- **Sub-Agent Research 4+**: Integration pattern analysis (needs codebase patterns complete)
- **Sub-Agent Research 5+**: Implementation strategy research (needs external research complete)
- **Sub-Agent Research 6+**: Testing strategy research (needs codebase and external research)

**MANDATORY: Use unlimited sub-agent spawning for research - let dependencies naturally control flow**

### Critical Context to Include and pass to the AI agent
Using research findings, ensure the following context is collected:

- **Documentation**: URLs with specific sections
- **Code Examples**: Real snippets from codebase
- **Gotchas**: Library quirks, version issues
- **Patterns**: Existing approaches to follow
- **Dependency Maps**: Clear task dependency graphs for sub-agent spawning decisions

## Phase 2: Dynamic Implementation Blueprint Generation

### ULTRATHINK WITH DYNAMIC DEPENDENCY PLANNING
- Think hard before execution. Create comprehensive plan addressing all requirements.
- **CRITICAL: Map out complete task dependency graph - identify which tasks can start immediately vs need prerequisites**
- **Create dependency completion triggers for continuous sub-agent spawning**
- **Plan for unlimited sub-agent spawning - spawn as many as there are ready-to-execute tasks**
- **Design for dynamic sub-agent spawning as dependencies complete and new tasks become available**

### Implementation Blueprint FOR DYNAMIC SUB-AGENT EXECUTION
- Start with pseudocode showing approach **designed for dynamic sub-agent delegation**
- Reference real files for patterns with exact paths for Claude Code
- Include error handling strategy for dynamic sub-agent coordination
- **MANDATORY: List tasks to be completed to fulfill the requirements in the order they should be completed**
- **CRITICAL: Map out task dependencies and identify waves of sub-agent spawning opportunities**
- **Structure tasks with clear dependency completion triggers for dynamic sub-agent spawning**

### Validation Gates (Must be Executable AND Dependency-Aware Sub-Agent Spawnable) eg for python
```bash
# Wave 1: Independent validation (spawn immediately when code ready)
# Syntax/Style
ruff check --fix && mypy .

# Wave 2: Dependency-triggered validation
# Unit Tests  
uv run pytest tests/ -v

# Sub-agent specific parallel validation:
# Sub-agent 1: Syntax checking (no dependencies)
ruff check --fix src/ &

# Sub-agent 2: Formatting (no dependencies) 
ruff format src/ &

# Sub-agent 3: Type checking (needs clean syntax)
mypy src/ --check-untyped-defs &

# Sub-agent 4: Unit tests (needs type-safe code)
uv run pytest tests/unit/ -v &

# Sub-agent 5: Integration tests (needs unit tests passing)
uv run pytest tests/integration/ -v &
```

*** CRITICAL AFTER YOU ARE DONE RESEARCHING AND EXPLORING THE CODEBASE BEFORE YOU START WRITING THE IMPLEMENTATION PLAN ***

*** ULTRATHINK ABOUT THE APPROACH AND PLAN YOUR CLAUDE CODE DYNAMIC SUB-AGENT STRATEGY THEN START IMPLEMENTATION ***

## Phase 3: Execute with DYNAMIC SUB-AGENT SPAWNING

### Load and Understand Requirements
- Read and understand all context and requirements from research phase
- Follow all instructions and extend the research if needed using additional sub-agent spawning
- Ensure you have all needed context to implement the feature fully
- Do more web searches and codebase exploration as needed using sub-agents

### ULTRATHINK Phase
- Think hard before you execute the plan. Create a comprehensive plan addressing all requirements.
- Break down complex tasks into smaller, manageable steps for sub-agent delegation
- Use task tracking to create and track your implementation plan
- Identify implementation patterns from existing code to follow

### Immediate Execution Pattern
1. **Initial Wave**: Spawn sub-agents for all tasks with no dependencies
2. **Completion Triggers**: As sub-agents complete, immediately check for newly available tasks
3. **Dynamic Spawning**: Spawn new sub-agents for any task whose dependencies are now satisfied
4. **Continuous Process**: Repeat until all tasks complete
5. **No Artificial Limits**: Let task dependencies naturally limit parallelism, not arbitrary agent counts

### Execute the Implementation Plan
- Execute using **DYNAMIC DEPENDENCY-DRIVEN SUB-AGENT PROCESSING**
- **MANDATORY: Spawn sub-agents immediately for all tasks with satisfied dependencies**
- **CONTINUOUSLY spawn new sub-agents as prerequisite tasks complete and dependencies are satisfied**
- **DO NOT wait for sub-agents to complete if other independent tasks can start - spawn more sub-agents**
- **Let natural dependencies limit parallelism - spawn unlimited sub-agents for ready tasks**
- **Monitor dependency completion and dynamically spawn sub-agents for newly available tasks**
- Implement all the code using coordinated sub-agent execution

### Dynamic Validation with Dependency-Aware Sub-Agents
- **Run each validation command** using appropriate sub-agents
- **Fix any failures** using debugging sub-agents while other validations continue
- **Re-run until all pass** using coordinated sub-agent re-validation
- Spawn validation sub-agents as soon as components are ready for testing
- Use dependency-aware validation - test components as their dependencies complete
- Continuously spawn new validation sub-agents as more components become ready

## Phase 4: Complete and Validate

### Completion Requirements
- Ensure all checklist items done using dynamic sub-agent coordination
- Run final validation suite leveraging unlimited sub-agent parallel execution
- Report completion status with sub-agent spawning metrics and dependency resolution stats
- **Read the feature requirements again to ensure you have implemented everything**
- Verify all original feature requirements have been implemented

### Reference and Verification
- You can always reference the original feature file again if needed
- Use sub-agents for final verification and cross-checking
- **Note: If validation fails, use error patterns to fix and retry using coordinated sub-agents**

### Quality Checklist WITH DYNAMIC SUB-AGENT REQUIREMENTS
- [ ] All necessary context included from comprehensive research
- [ ] Validation gates are executable by Claude Code **and structured for dependency-triggered sub-agent waves**
- [ ] References existing patterns from codebase analysis
- [ ] Clear implementation path **designed for unlimited dynamic sub-agent spawning**
- [ ] Error handling documented **for dynamic sub-agent coordination**
- [ ] **Complete dependency graph created for dynamic sub-agent spawning**
- [ ] **Clear identification of immediate-start tasks vs dependency-gated tasks**
- [ ] **Task breakdown designed for unlimited dynamic sub-agent spawning**
- [ ] **All feature requirements implemented using sub-agent coordination**
- [ ] **Sub-agent spawning metrics and performance reported**

Score the unified implementation on a scale of 1-10 (confidence level to succeed in one-pass end-to-end execution using Claude Code **with unlimited dynamic sub-agent spawning**)

Remember: The goal is seamless one-pass feature implementation from comprehensive research through completion using **thorough context gathering AND unlimited dynamic sub-agent execution driven by task dependencies, not artificial constraints**.