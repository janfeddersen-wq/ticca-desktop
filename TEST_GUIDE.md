# Ticca Desktop Testing Guide

🐕 *Woof!* This guide documents the testing strategy, procedures, and best practices for the Ticca Desktop project.

## Table of Contents

1. [Overview](#overview)
2. [Test Architecture](#test-architecture)
3. [Running Tests](#running-tests)
4. [Test Coverage](#test-coverage)
5. [Writing Tests](#writing-tests)
6. [Quality Gates](#quality-gates)
7. [Performance Testing](#performance-testing)
8. [Security Testing](#security-testing)
9. [CI/CD Integration](#cicd-integration)

## Overview

Ticca Desktop is a hybrid Rust + Python application. Our testing strategy follows the test pyramid:

```
           ╔═══════════════╗
          ╔╝    E2E (10%)  ╚╗
         ╔╝    Integration   ╚╗
        ╔╝      (20%)         ╚╗
       ╔╝     Unit Tests       ╚╗
      ╔╝        (70%)           ╚╗
     ╚══════════════════════════╝
```

### Test Distribution

| Layer | Rust | Python | Coverage Target |
|-------|------|--------|-----------------|
| Unit Tests | `crates/*/src/*.rs` | `python/tests/test_*.py` | >90% |
| Integration | `tests/integration/` | `python/tests/test_*_integration.py` | >80% |
| E2E | Manual / UI automation | Playwright/Selenium | >70% critical paths |

## Test Architecture

### Rust Test Locations

```
crates/
├── ticca-config/src/
│   ├── settings.rs       # Settings tests (6 tests)
│   ├── loader.rs         # Config loading (4 tests)
│   ├── api_keys.rs       # API key management (5 tests)
│   ├── model_pinning.rs  # Model pinning (6 tests)
│   └── paths.rs          # Path handling (3 tests)
├── ticca-db/src/
│   ├── repositories/     # Repository tests
│   ├── migrations.rs     # Migration tests
│   └── schema.rs         # Schema tests
├── ticca-bridge/src/
│   ├── lib.rs            # Bridge tests (3 tests)
│   ├── types.rs          # Type serialization (4 tests)
│   └── error.rs          # Error handling (2 tests)
├── ticca-core/src/
│   └── error.rs          # Error tests (3 tests)
└── ticca-ui/src/
    └── main.rs           # UI tests (3 tests)
```

### Python Test Locations

```
python/tests/
├── conftest.py           # Shared fixtures
├── test_bridge.py        # Bridge interface tests
├── test_mcp.py           # MCP server tests
├── test_session.py       # Session management tests
├── test_file_ops.py      # File operations tests
├── test_shell.py         # Shell command tests
└── test_tokens.py        # Token estimation tests
```

## Running Tests

### Rust Tests

```bash
# Run all Rust tests
cargo test

# Run tests for specific crate
cargo test -p ticca-config
cargo test -p ticca-db
cargo test -p ticca-bridge
cargo test -p ticca-core
cargo test -p ticca-ui

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_default_settings

# Run tests with coverage (requires cargo-tarpaulin)
cargo tarpaulin -o html
```

### Python Tests

```bash
# Navigate to Python directory
cd python

# Install dev dependencies
pip install -e ".[dev]"

# Run all tests
pytest

# Run with coverage
pytest --cov=ticca_agent --cov-report=html

# Run specific test file
pytest tests/test_bridge.py

# Run specific test class/method
pytest tests/test_file_ops.py::TestFileOperationsSandbox

# Run only security tests
pytest -m security

# Run excluding slow tests
pytest -m "not slow"

# Run integration tests only
pytest -m integration

# Run with verbose output
pytest -v --tb=long

# Run and stop on first failure
pytest -x
```

### Test Markers

- `@pytest.mark.slow` - Long-running tests (>5s)
- `@pytest.mark.integration` - Integration tests
- `@pytest.mark.security` - Security-related tests

## Test Coverage

### Current Coverage Status

| Module | Coverage | Target |
|--------|----------|--------|
| `ticca_agent.bridge` | ~85% | 90% |
| `ticca_agent.session` | ~80% | 90% |
| `ticca_agent.mcp` | ~75% | 85% |
| `ticca_agent.tools` | ~70% | 90% |
| `ticca_agent.utils` | ~80% | 90% |

### Coverage Goals

1. **Critical Security Code**: >95% branch coverage
   - `tools/file_ops.py` - File sandbox
   - `tools/shell.py` - Shell execution
   
2. **Core Business Logic**: >90% coverage
   - Session management
   - Context compaction
   - Bridge interface

3. **Infrastructure**: >80% coverage
   - MCP server management
   - Provider interfaces

### Generating Coverage Reports

```bash
# Python coverage
cd python
pytest --cov=ticca_agent --cov-report=html --cov-report=term-missing
open htmlcov/index.html

# Rust coverage
cargo tarpaulin -o html
open tarpaulin-report.html
```

## Writing Tests

### Test Structure (Python)

```python
"""Tests for [module name].

These tests cover:
1. [Feature 1]
2. [Feature 2]
3. [Edge cases]
"""

import pytest
from ticca_agent.module import TargetClass


class TestTargetClass:
    """Tests for TargetClass."""
    
    def test_basic_functionality(self) -> None:
        """Test that [describe expected behavior]."""
        # Arrange
        input_data = ...
        
        # Act
        result = TargetClass().method(input_data)
        
        # Assert
        assert result == expected

    @pytest.mark.asyncio
    async def test_async_operation(self) -> None:
        """Test async operation."""
        result = await async_method()
        assert result is not None

    @pytest.mark.security
    def test_security_boundary(self) -> None:
        """Test that security boundary is enforced."""
        with pytest.raises(PermissionError):
            dangerous_operation()
```

### Test Structure (Rust)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        // Arrange
        let input = TestInput::new();
        
        // Act
        let result = process(input);
        
        // Assert
        assert_eq!(result, expected);
    }

    #[test]
    fn test_error_case() {
        let result = process_invalid();
        assert!(result.is_err());
    }
}
```

### Best Practices

1. **Test Naming**: `test_[scenario]_[expected_behavior]`
2. **One Assert Per Concept**: Keep tests focused
3. **Use Fixtures**: Share setup via conftest.py
4. **Test Edge Cases**: Empty inputs, boundaries, errors
5. **Document Tests**: Clear docstrings explaining purpose

## Quality Gates

### Pre-Commit Checks

```yaml
# Run before committing:
- cargo fmt --check
- cargo clippy
- cargo test
- ruff check python/
- mypy python/ticca_agent/
- pytest python/tests/
```

### PR Requirements

1. ✅ All existing tests pass
2. ✅ New code has accompanying tests
3. ✅ Coverage doesn't decrease
4. ✅ No new clippy/ruff warnings
5. ✅ Type checking passes (mypy)

### Coverage Requirements

| Change Type | Requirement |
|-------------|-------------|
| New feature | >90% coverage for new code |
| Bug fix | Test case reproducing bug |
| Refactor | No coverage decrease |
| Security | >95% branch coverage |

## Performance Testing

### UI Performance (120 FPS Target)

**Test Scenarios:**

1. **Initial Render**
   - Target: <16ms first meaningful paint
   - Measure: Time from app launch to interactive

2. **Message Streaming**
   - Target: No frame drops during AI streaming
   - Measure: Frame timing during token rendering

3. **Scroll Performance**
   - Target: 60+ FPS during scroll
   - Measure: Jank-free scrolling with 1000+ messages

4. **Memory Under Load**
   - Target: <500MB for normal usage
   - Measure: Memory after extended session

### Load Testing (AI Processing)

```bash
# Using k6 for API load testing (if exposing HTTP API)
k6 run --vus 10 --duration 30s performance/load_test.js

# Manual test: concurrent sessions
# Open 5+ sessions simultaneously
# Send messages in parallel
# Monitor memory and responsiveness
```

### Performance Test Cases

| Test | Metric | Target | Critical |
|------|--------|--------|----------|
| App startup | Time to interactive | <2s | Yes |
| Message render | Frame time | <16ms | Yes |
| Long conversation | Memory growth | <10MB/100 msgs | Yes |
| File listing | Response time | <500ms | No |
| Grep search | Response time | <2s | No |

## Security Testing

### Critical Security Tests

1. **Path Traversal Prevention**
   - Location: `tests/test_file_ops.py::TestFileOperationsSandbox`
   - Tests sandbox validation
   - Blocks `../` attacks
   - Blocks symlink escapes

2. **Shell Command Safety**
   - Location: `tests/test_shell.py`
   - Timeout enforcement
   - Output sanitization
   - Working directory validation

3. **Input Validation**
   - All tool inputs validated
   - JSON schema enforcement
   - Type checking

### Running Security Tests

```bash
# Run security-marked tests
pytest -m security -v

# Run with bandit (Python security scanner)
pip install bandit
bandit -r python/ticca_agent/

# Rust security audit
cargo audit
```

### Security Test Checklist

- [x] File sandbox blocks path traversal
- [x] Shell commands respect timeout
- [x] API keys not logged/exposed
- [x] Session data encrypted at rest (TODO)
- [ ] HTTPS enforcement for MCP SSE
- [ ] Rate limiting for providers

## CI/CD Integration

### GitHub Actions Workflow

```yaml
name: Test Suite

on: [push, pull_request]

jobs:
  rust-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --all-features
      - run: cargo clippy -- -D warnings

  python-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: '3.11'
      - run: pip install -e "python/[dev]"
      - run: pytest python/tests/ --cov --cov-report=xml
      - uses: codecov/codecov-action@v4
```

### Quality Gate Matrix

| Gate | Trigger | Required |
|------|---------|----------|
| Unit tests | Every commit | Yes |
| Lint (cargo clippy) | Every commit | Yes |
| Lint (ruff) | Every commit | Yes |
| Type check (mypy) | Every commit | Yes |
| Integration tests | PR merge | Yes |
| Coverage report | PR | No (informational) |
| Security scan | Weekly | Yes |

## Appendix

### Test Data Factories

```python
# python/tests/conftest.py provides:
- temp_dir: Temporary directory
- temp_file: Test file with content
- nested_dir_structure: Complex directory tree
- sample_messages: List of Message objects
- mock_provider: Mock AI provider
- session_config: Session configuration
```

### Common Test Patterns

**Async Testing:**
```python
@pytest.mark.asyncio
async def test_async_method() -> None:
    result = await async_operation()
    assert result.success
```

**Exception Testing:**
```python
def test_raises_on_invalid() -> None:
    with pytest.raises(ValueError, match="expected error"):
        invalid_operation()
```

**Parametrized Tests:**
```python
@pytest.mark.parametrize("input,expected", [
    ("valid", True),
    ("invalid", False),
])
def test_validation(input: str, expected: bool) -> None:
    assert validate(input) == expected
```

### Debugging Test Failures

```bash
# Show full traceback
pytest --tb=long

# Drop into debugger on failure
pytest --pdb

# Run only failed tests from last run
pytest --lf

# Show local variables in traceback
pytest --showlocals
```

---

*Last updated: Phase 11 QA Testing*
*Maintained by: QA Puppy 🐕*
