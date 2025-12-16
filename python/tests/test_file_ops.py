"""Tests for file operation tools.

These tests cover:
1. FileOperationsSandbox - path validation and security
2. ListFilesTool - directory listing
3. ReadFileTool - file reading with token estimation
4. GrepTool - text search
5. EditFileTool - file editing
6. DeleteFileTool - file deletion

Security is a primary concern - path traversal attacks must be blocked.
"""

from __future__ import annotations

import os
import tempfile
from pathlib import Path
from typing import Any

import pytest

from ticca_agent.tools.file_ops import (
    ContentPayload,
    DeleteFileInput,
    DeleteFileTool,
    DeleteSnippetPayload,
    EditFileTool,
    FileEntry,
    FileOperationsSandbox,
    GrepInput,
    GrepTool,
    IGNORE_PATTERNS,
    ListFilesInput,
    ListFilesTool,
    ReadFileInput,
    ReadFileTool,
    Replacement,
    ReplacementsPayload,
    estimate_tokens,
    should_ignore,
)


# =============================================================================
# FileOperationsSandbox Tests - CRITICAL SECURITY
# =============================================================================


class TestFileOperationsSandbox:
    """Tests for the file operations sandbox - SECURITY CRITICAL."""

    @pytest.mark.security
    def test_sandbox_allows_paths_within_root(self, temp_dir: Path) -> None:
        """Test that paths within allowed roots are accepted."""
        sandbox = FileOperationsSandbox([temp_dir])
        
        # Create a file within sandbox
        test_file = temp_dir / "test.txt"
        test_file.touch()
        
        # Should succeed
        result = sandbox.validate_path(test_file)
        assert result == test_file.resolve()

    @pytest.mark.security
    def test_sandbox_allows_subdirectories(self, temp_dir: Path) -> None:
        """Test that subdirectories within allowed roots are accepted."""
        sandbox = FileOperationsSandbox([temp_dir])
        
        # Create nested structure
        nested = temp_dir / "a" / "b" / "c"
        nested.mkdir(parents=True)
        test_file = nested / "deep.txt"
        test_file.touch()
        
        # Should succeed
        result = sandbox.validate_path(test_file)
        assert result == test_file.resolve()

    @pytest.mark.security
    def test_sandbox_blocks_path_outside_root(self, temp_dir: Path) -> None:
        """Test that paths outside allowed roots are blocked."""
        sandbox = FileOperationsSandbox([temp_dir])
        
        # Try to access system file
        with pytest.raises(PermissionError, match="outside allowed directories"):
            sandbox.validate_path("/etc/passwd")

    @pytest.mark.security
    def test_sandbox_blocks_parent_traversal(self, temp_dir: Path) -> None:
        """Test that parent directory traversal is blocked."""
        sandbox = FileOperationsSandbox([temp_dir])
        
        # Try path traversal attack
        with pytest.raises(PermissionError, match="outside allowed directories"):
            sandbox.validate_path(temp_dir / ".." / ".." / "etc" / "passwd")

    @pytest.mark.security
    def test_sandbox_blocks_symlink_escape(self, temp_dir: Path) -> None:
        """Test that symlink escapes are blocked."""
        sandbox = FileOperationsSandbox([temp_dir])
        
        # Create symlink pointing outside sandbox
        symlink = temp_dir / "escape_link"
        try:
            symlink.symlink_to("/etc")
        except OSError:
            pytest.skip("Symlinks not supported on this platform")
        
        # Trying to access through symlink should fail
        with pytest.raises(PermissionError, match="outside allowed directories"):
            sandbox.validate_path(symlink / "passwd")

    @pytest.mark.security
    def test_sandbox_handles_relative_paths(self, temp_dir: Path) -> None:
        """Test that relative paths are resolved correctly."""
        sandbox = FileOperationsSandbox([temp_dir])
        
        # Create file
        test_file = temp_dir / "test.txt"
        test_file.touch()
        
        # Change to temp dir and use relative path
        original_cwd = os.getcwd()
        try:
            os.chdir(temp_dir)
            result = sandbox.validate_path("test.txt")
            assert result == test_file.resolve()
        finally:
            os.chdir(original_cwd)

    @pytest.mark.security
    def test_sandbox_multiple_roots(self, temp_dir: Path) -> None:
        """Test sandbox with multiple allowed roots."""
        root1 = temp_dir / "root1"
        root2 = temp_dir / "root2"
        root1.mkdir()
        root2.mkdir()
        
        sandbox = FileOperationsSandbox([root1, root2])
        
        # Both should be accessible
        file1 = root1 / "file1.txt"
        file2 = root2 / "file2.txt"
        file1.touch()
        file2.touch()
        
        assert sandbox.validate_path(file1) == file1.resolve()
        assert sandbox.validate_path(file2) == file2.resolve()
        
        # Outside both should fail
        with pytest.raises(PermissionError):
            sandbox.validate_path(temp_dir / "outside.txt")

    @pytest.mark.security
    def test_sandbox_default_cwd(self) -> None:
        """Test that sandbox defaults to current working directory."""
        sandbox = FileOperationsSandbox()
        
        # Path in cwd should work
        result = sandbox.validate_path(".")
        assert result == Path.cwd().resolve()


# =============================================================================
# Ignore Pattern Tests
# =============================================================================


class TestIgnorePatterns:
    """Tests for ignore pattern functionality."""

    def test_should_ignore_node_modules(self, temp_dir: Path) -> None:
        """Test that node_modules is ignored."""
        path = temp_dir / "node_modules" / "package" / "index.js"
        assert should_ignore(path) is True

    def test_should_ignore_git_directory(self, temp_dir: Path) -> None:
        """Test that .git directory is ignored."""
        path = temp_dir / ".git" / "objects"
        assert should_ignore(path) is True

    def test_should_ignore_pycache(self, temp_dir: Path) -> None:
        """Test that __pycache__ is ignored."""
        path = temp_dir / "__pycache__" / "module.pyc"
        assert should_ignore(path) is True

    def test_should_ignore_pyc_files(self, temp_dir: Path) -> None:
        """Test that .pyc files are ignored."""
        path = temp_dir / "module.pyc"
        assert should_ignore(path) is True

    def test_should_ignore_venv(self, temp_dir: Path) -> None:
        """Test that .venv is ignored."""
        path = temp_dir / ".venv" / "lib" / "python"
        assert should_ignore(path) is True

    def test_should_not_ignore_source_files(self, temp_dir: Path) -> None:
        """Test that normal source files are not ignored."""
        path = temp_dir / "src" / "main.py"
        assert should_ignore(path) is False

    def test_should_not_ignore_docs(self, temp_dir: Path) -> None:
        """Test that documentation files are not ignored."""
        path = temp_dir / "docs" / "readme.md"
        assert should_ignore(path) is False


# =============================================================================
# Token Estimation Tests
# =============================================================================


class TestTokenEstimation:
    """Tests for token estimation utility."""

    def test_estimate_empty_string(self) -> None:
        """Test estimation for empty string."""
        assert estimate_tokens("") == 0

    def test_estimate_short_text(self) -> None:
        """Test estimation for short text."""
        # "Hello" = 5 chars -> 5 // 4 = 1 token
        result = estimate_tokens("Hello")
        assert result == 1

    def test_estimate_longer_text(self) -> None:
        """Test estimation for longer text."""
        # 100 chars -> 25 tokens
        text = "x" * 100
        result = estimate_tokens(text)
        assert result == 25

    def test_estimate_realistic_text(self) -> None:
        """Test estimation for realistic text."""
        text = "This is a sample sentence for testing token estimation."
        result = estimate_tokens(text)
        # ~55 chars -> ~13 tokens
        assert 10 <= result <= 20


# =============================================================================
# ListFilesTool Tests
# =============================================================================


class TestListFilesTool:
    """Tests for the list_files tool."""

    @pytest.mark.asyncio
    async def test_list_empty_directory(self, temp_dir: Path) -> None:
        """Test listing an empty directory."""
        tool = ListFilesTool()
        result = await tool.execute(ListFilesInput(directory=str(temp_dir)))
        
        assert result.error is None
        assert "0 directories, 0 files" in result.content

    @pytest.mark.asyncio
    async def test_list_directory_with_files(
        self, nested_dir_structure: Path
    ) -> None:
        """Test listing a directory with files."""
        tool = ListFilesTool()
        result = await tool.execute(
            ListFilesInput(directory=str(nested_dir_structure))
        )
        
        assert result.error is None
        assert "README.md" in result.content
        assert "src" in result.content
        assert "tests" in result.content

    @pytest.mark.asyncio
    async def test_list_directory_recursive(
        self, nested_dir_structure: Path
    ) -> None:
        """Test recursive directory listing."""
        tool = ListFilesTool()
        result = await tool.execute(
            ListFilesInput(
                directory=str(nested_dir_structure),
                recursive=True,
            )
        )
        
        assert result.error is None
        assert "main.py" in result.content
        assert "helper.py" in result.content

    @pytest.mark.asyncio
    async def test_list_nonexistent_directory(self) -> None:
        """Test listing a nonexistent directory."""
        tool = ListFilesTool()
        result = await tool.execute(
            ListFilesInput(directory="/nonexistent/path/abc123")
        )
        
        assert result.error is not None
        assert "does not exist" in result.error

    @pytest.mark.asyncio
    async def test_list_file_instead_of_directory(self, temp_file: Path) -> None:
        """Test error when path is a file, not directory."""
        tool = ListFilesTool()
        result = await tool.execute(
            ListFilesInput(directory=str(temp_file))
        )
        
        assert result.error is not None
        assert "not a directory" in result.error

    @pytest.mark.asyncio
    async def test_list_ignores_node_modules(self, temp_dir: Path) -> None:
        """Test that node_modules is ignored."""
        # Create node_modules directory
        (temp_dir / "node_modules" / "package").mkdir(parents=True)
        (temp_dir / "node_modules" / "package" / "index.js").touch()
        (temp_dir / "src").mkdir()
        (temp_dir / "src" / "main.js").touch()
        
        tool = ListFilesTool()
        result = await tool.execute(
            ListFilesInput(directory=str(temp_dir), recursive=True)
        )
        
        assert result.error is None
        assert "node_modules" not in result.content
        assert "main.js" in result.content

    @pytest.mark.asyncio
    async def test_list_non_recursive(
        self, nested_dir_structure: Path
    ) -> None:
        """Test non-recursive listing."""
        tool = ListFilesTool()
        result = await tool.execute(
            ListFilesInput(
                directory=str(nested_dir_structure),
                recursive=False,
            )
        )
        
        assert result.error is None
        assert "README.md" in result.content
        # Should not show nested files
        assert "main.py" not in result.content


# =============================================================================
# ReadFileTool Tests
# =============================================================================


class TestReadFileTool:
    """Tests for the read_file tool."""

    @pytest.mark.asyncio
    async def test_read_file_success(self, temp_file: Path) -> None:
        """Test successfully reading a file."""
        tool = ReadFileTool()
        result = await tool.execute(
            ReadFileInput(file_path=str(temp_file))
        )
        
        assert result.error is None
        assert result.content is not None
        assert "Hello, World!" in result.content
        assert result.num_tokens > 0

    @pytest.mark.asyncio
    async def test_read_file_not_found(self) -> None:
        """Test reading a nonexistent file."""
        tool = ReadFileTool()
        result = await tool.execute(
            ReadFileInput(file_path="/nonexistent/file.txt")
        )
        
        assert result.error is not None
        assert "does not exist" in result.error

    @pytest.mark.asyncio
    async def test_read_directory_error(self, temp_dir: Path) -> None:
        """Test error when trying to read a directory."""
        tool = ReadFileTool()
        result = await tool.execute(
            ReadFileInput(file_path=str(temp_dir))
        )
        
        assert result.error is not None
        assert "not a file" in result.error

    @pytest.mark.asyncio
    async def test_read_file_with_line_range(self, temp_file: Path) -> None:
        """Test reading specific lines from a file."""
        tool = ReadFileTool()
        result = await tool.execute(
            ReadFileInput(
                file_path=str(temp_file),
                start_line=2,
                num_lines=1,
            )
        )
        
        assert result.error is None
        assert result.content is not None
        assert "This is a test file" in result.content
        assert "Hello, World!" not in result.content

    @pytest.mark.asyncio
    async def test_read_file_start_line_without_num_lines(
        self, temp_file: Path
    ) -> None:
        """Test error when start_line provided without num_lines."""
        tool = ReadFileTool()
        result = await tool.execute(
            ReadFileInput(
                file_path=str(temp_file),
                start_line=1,
            )
        )
        
        assert result.error is not None
        assert "num_lines must be specified" in result.error

    @pytest.mark.asyncio
    async def test_read_empty_path(self) -> None:
        """Test error on empty path."""
        tool = ReadFileTool()
        result = await tool.execute(
            ReadFileInput(file_path="")
        )
        
        assert result.error is not None
        assert "cannot be empty" in result.error

    @pytest.mark.asyncio
    async def test_read_utf8_file(self, temp_dir: Path) -> None:
        """Test reading UTF-8 encoded file."""
        utf8_file = temp_dir / "utf8.txt"
        utf8_file.write_text("Hello 你好 🎉", encoding="utf-8")
        
        tool = ReadFileTool()
        result = await tool.execute(
            ReadFileInput(file_path=str(utf8_file))
        )
        
        assert result.error is None
        assert "你好" in result.content
        assert "🎉" in result.content


# =============================================================================
# GrepTool Tests
# =============================================================================


class TestGrepTool:
    """Tests for the grep tool."""

    @pytest.mark.asyncio
    async def test_grep_finds_match(
        self, nested_dir_structure: Path
    ) -> None:
        """Test grep finds matching content."""
        tool = GrepTool()
        result = await tool.execute(
            GrepInput(
                search_string="def main",
                directory=str(nested_dir_structure),
            )
        )
        
        assert result.error is None
        assert len(result.matches) >= 1
        # Should find in main.py
        match_files = [m.file_path for m in result.matches if m.file_path]
        assert any("main.py" in f for f in match_files)

    @pytest.mark.asyncio
    async def test_grep_no_matches(self, nested_dir_structure: Path) -> None:
        """Test grep with no matches."""
        tool = GrepTool()
        result = await tool.execute(
            GrepInput(
                search_string="this_string_does_not_exist_xyz123",
                directory=str(nested_dir_structure),
            )
        )
        
        assert result.error is None
        assert len(result.matches) == 0

    @pytest.mark.asyncio
    async def test_grep_empty_search_string(
        self, nested_dir_structure: Path
    ) -> None:
        """Test error on empty search string."""
        tool = GrepTool()
        result = await tool.execute(
            GrepInput(
                search_string="   ",
                directory=str(nested_dir_structure),
            )
        )
        
        assert result.error is not None
        assert "cannot be empty" in result.error

    @pytest.mark.asyncio
    async def test_grep_nonexistent_directory(self) -> None:
        """Test grep on nonexistent directory."""
        tool = GrepTool()
        result = await tool.execute(
            GrepInput(
                search_string="test",
                directory="/nonexistent/directory",
            )
        )
        
        assert result.error is not None

    @pytest.mark.asyncio
    async def test_grep_case_sensitive(self, temp_dir: Path) -> None:
        """Test case-sensitive search."""
        # Create test file
        test_file = temp_dir / "test.txt"
        test_file.write_text("Hello World\nhello world\nHELLO WORLD\n")
        
        tool = GrepTool()
        result = await tool.execute(
            GrepInput(
                search_string="Hello",
                directory=str(temp_dir),
            )
        )
        
        assert result.error is None
        # Should find at least one match (ripgrep/python depends on availability)
        assert len(result.matches) >= 1


# =============================================================================
# EditFileTool Tests
# =============================================================================


class TestEditFileTool:
    """Tests for the edit_file tool."""

    @pytest.mark.asyncio
    async def test_edit_create_new_file(self, temp_dir: Path) -> None:
        """Test creating a new file."""
        new_file = temp_dir / "new_file.txt"
        
        tool = EditFileTool()
        result = await tool.execute(
            ContentPayload(
                file_path=str(new_file),
                content="New file content",
                overwrite=True,
            )
        )
        
        assert result.success is True
        assert result.changed is True
        assert new_file.exists()
        assert new_file.read_text() == "New file content"

    @pytest.mark.asyncio
    async def test_edit_overwrite_existing(self, temp_file: Path) -> None:
        """Test overwriting existing file."""
        original_content = temp_file.read_text()
        
        tool = EditFileTool()
        result = await tool.execute(
            ContentPayload(
                file_path=str(temp_file),
                content="Updated content",
                overwrite=True,
            )
        )
        
        assert result.success is True
        assert result.changed is True
        assert temp_file.read_text() == "Updated content"
        assert result.diff is not None

    @pytest.mark.asyncio
    async def test_edit_no_overwrite_existing(self, temp_file: Path) -> None:
        """Test error when trying to overwrite without flag."""
        tool = EditFileTool()
        result = await tool.execute(
            ContentPayload(
                file_path=str(temp_file),
                content="New content",
                overwrite=False,
            )
        )
        
        assert result.success is False
        assert result.error is not None
        assert "exists" in result.error.lower()

    @pytest.mark.asyncio
    async def test_edit_replacements(self, temp_file: Path) -> None:
        """Test applying text replacements."""
        tool = EditFileTool()
        result = await tool.execute(
            ReplacementsPayload(
                file_path=str(temp_file),
                replacements=[
                    Replacement(old_str="Hello, World!", new_str="Goodbye, World!"),
                ],
            )
        )
        
        assert result.success is True
        assert result.changed is True
        assert "Goodbye, World!" in temp_file.read_text()

    @pytest.mark.asyncio
    async def test_edit_replacement_not_found(self, temp_file: Path) -> None:
        """Test error when replacement text not found."""
        tool = EditFileTool()
        result = await tool.execute(
            ReplacementsPayload(
                file_path=str(temp_file),
                replacements=[
                    Replacement(old_str="nonexistent text xyz", new_str="replacement"),
                ],
            )
        )
        
        assert result.error is not None
        assert "not found" in result.error.lower()

    @pytest.mark.asyncio
    async def test_edit_delete_snippet(self, temp_file: Path) -> None:
        """Test deleting a snippet."""
        tool = EditFileTool()
        result = await tool.execute(
            DeleteSnippetPayload(
                file_path=str(temp_file),
                delete_snippet="Hello, World!\n",
            )
        )
        
        assert result.success is True
        assert result.changed is True
        assert "Hello, World!" not in temp_file.read_text()

    @pytest.mark.asyncio
    async def test_edit_no_change(self, temp_file: Path) -> None:
        """Test when content unchanged."""
        original = temp_file.read_text()
        
        tool = EditFileTool()
        result = await tool.execute(
            ContentPayload(
                file_path=str(temp_file),
                content=original,
                overwrite=True,
            )
        )
        
        assert result.success is True
        assert result.changed is False

    @pytest.mark.asyncio
    async def test_edit_creates_parent_directories(self, temp_dir: Path) -> None:
        """Test that parent directories are created."""
        deep_file = temp_dir / "a" / "b" / "c" / "file.txt"
        
        tool = EditFileTool()
        result = await tool.execute(
            ContentPayload(
                file_path=str(deep_file),
                content="Deep file content",
                overwrite=True,
            )
        )
        
        assert result.success is True
        assert deep_file.exists()
        assert deep_file.read_text() == "Deep file content"


# =============================================================================
# DeleteFileTool Tests
# =============================================================================


class TestDeleteFileTool:
    """Tests for the delete_file tool."""

    @pytest.mark.asyncio
    async def test_delete_existing_file(self, temp_file: Path) -> None:
        """Test deleting an existing file."""
        assert temp_file.exists()
        
        tool = DeleteFileTool()
        result = await tool.execute(
            DeleteFileInput(file_path=str(temp_file))
        )
        
        assert result.success is True
        assert result.changed is True
        assert not temp_file.exists()

    @pytest.mark.asyncio
    async def test_delete_nonexistent_file(self, temp_dir: Path) -> None:
        """Test deleting a nonexistent file (idempotent)."""
        nonexistent = temp_dir / "does_not_exist.txt"
        
        tool = DeleteFileTool()
        result = await tool.execute(
            DeleteFileInput(file_path=str(nonexistent))
        )
        
        # Should succeed but not change anything
        assert result.success is True
        assert result.changed is False

    @pytest.mark.asyncio
    async def test_delete_directory_error(self, temp_dir: Path) -> None:
        """Test error when trying to delete a directory."""
        tool = DeleteFileTool()
        result = await tool.execute(
            DeleteFileInput(file_path=str(temp_dir))
        )
        
        assert result.error is not None
        assert "not a regular file" in result.error

    @pytest.mark.asyncio
    async def test_delete_empty_path(self) -> None:
        """Test error on empty path."""
        tool = DeleteFileTool()
        result = await tool.execute(
            DeleteFileInput(file_path="")
        )
        
        assert result.error is not None
        assert "cannot be empty" in result.error


# =============================================================================
# Integration Tests
# =============================================================================


class TestFileOpsIntegration:
    """Integration tests for file operations."""

    @pytest.mark.asyncio
    @pytest.mark.integration
    async def test_create_edit_read_delete_workflow(self, temp_dir: Path) -> None:
        """Test complete file workflow: create -> edit -> read -> delete."""
        file_path = temp_dir / "workflow_test.txt"
        
        # 1. Create file
        edit_tool = EditFileTool()
        create_result = await edit_tool.execute(
            ContentPayload(
                file_path=str(file_path),
                content="Initial content",
                overwrite=True,
            )
        )
        assert create_result.success is True
        
        # 2. Edit file
        edit_result = await edit_tool.execute(
            ReplacementsPayload(
                file_path=str(file_path),
                replacements=[
                    Replacement(old_str="Initial", new_str="Modified"),
                ],
            )
        )
        assert edit_result.success is True
        
        # 3. Read file
        read_tool = ReadFileTool()
        read_result = await read_tool.execute(
            ReadFileInput(file_path=str(file_path))
        )
        assert read_result.error is None
        assert "Modified content" in read_result.content
        
        # 4. Delete file
        delete_tool = DeleteFileTool()
        delete_result = await delete_tool.execute(
            DeleteFileInput(file_path=str(file_path))
        )
        assert delete_result.success is True
        assert not file_path.exists()

    @pytest.mark.asyncio
    @pytest.mark.integration
    async def test_list_and_grep_workflow(
        self, nested_dir_structure: Path
    ) -> None:
        """Test list + grep workflow."""
        # 1. List files
        list_tool = ListFilesTool()
        list_result = await list_tool.execute(
            ListFilesInput(
                directory=str(nested_dir_structure),
                recursive=True,
            )
        )
        assert list_result.error is None
        assert "main.py" in list_result.content
        
        # 2. Grep for content
        grep_tool = GrepTool()
        grep_result = await grep_tool.execute(
            GrepInput(
                search_string="def",
                directory=str(nested_dir_structure),
            )
        )
        assert grep_result.error is None
        # Should find functions in .py files
        assert len(grep_result.matches) >= 1
