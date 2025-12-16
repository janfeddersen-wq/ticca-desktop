"""File operation tools for Ticca Agent.

This module provides comprehensive file system tools for agents:
- list_files: Directory exploration with recursive support
- read_file: Read file contents with token estimation
- grep: Text search across files
- edit_file: Create, modify, and delete file content
- delete_file: Remove files permanently

All tools follow the BaseTool interface and include:
- Pydantic schema validation
- Token safety measures
- Comprehensive error handling
- Ignore pattern support for common artifacts

Example:
    >>> from ticca_agent.tools.file_ops import ListFilesTool
    >>> tool = ListFilesTool()
    >>> result = await tool.run(ListFilesInput(directory="."))
    >>> print(result.output)

"""

from __future__ import annotations

import difflib
import fnmatch
import os
import re
import subprocess
from pathlib import Path
from typing import TYPE_CHECKING, Any, ClassVar

from pydantic import BaseModel, Field

from ticca_agent.tools.base import BaseTool, ToolConfig, ToolExecutionError

if TYPE_CHECKING:
    from ticca_agent.core.context import RunContext


# Common directories and patterns to ignore during file operations
IGNORE_PATTERNS: tuple[str, ...] = (
    # Version control
    ".git",
    ".svn",
    ".hg",
    ".bzr",
    # Node.js
    "node_modules",
    ".npm",
    ".yarn",
    ".pnpm-store",
    # Python
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    "*.pyc",
    "*.pyo",
    ".venv",
    "venv",
    ".env",
    "env",
    ".tox",
    "*.egg-info",
    ".eggs",
    "dist",
    "build",
    ".nox",
    ".hypothesis",
    # IDE and editors
    ".idea",
    ".vscode",
    ".vs",
    "*.swp",
    "*.swo",
    "*~",
    ".project",
    ".settings",
    ".classpath",
    # Build outputs
    "target",
    "out",
    "output",
    "bin",
    "obj",
    ".next",
    ".nuxt",
    ".output",
    ".cache",
    ".parcel-cache",
    # Dependencies and packages
    "vendor",
    "bower_components",
    "jspm_packages",
    # Logs and databases
    "*.log",
    "*.sqlite",
    "*.sqlite3",
    "*.db",
    # OS generated
    ".DS_Store",
    "Thumbs.db",
    "desktop.ini",
    # Temporary files
    "*.tmp",
    "*.temp",
    "*.bak",
    ".tmp",
    "tmp",
    "temp",
    # Coverage
    "coverage",
    ".coverage",
    "htmlcov",
    ".nyc_output",
    "lcov.info",
    # Documentation builds
    "site",
    "_build",
    "docs/_build",
    # Rust
    "target/debug",
    "target/release",
    "Cargo.lock",
    # Go
    "go.sum",
    # Lock files (often large)
    "package-lock.json",
    "yarn.lock",
    "pnpm-lock.yaml",
    "poetry.lock",
    "Pipfile.lock",
    # Large media files
    "*.mp4",
    "*.mp3",
    "*.avi",
    "*.mov",
    "*.mkv",
    "*.wav",
    "*.flac",
    # Archives
    "*.zip",
    "*.tar",
    "*.tar.gz",
    "*.tgz",
    "*.rar",
    "*.7z",
    # Binary files
    "*.exe",
    "*.dll",
    "*.so",
    "*.dylib",
    "*.bin",
    # Images (unless specifically needed)
    "*.ico",
    "*.icns",
)

# Maximum results for various operations
MAX_GREP_MATCHES = 200
MAX_LIST_FILES = 5000
MAX_FILE_SIZE_BYTES = 10 * 1024 * 1024  # 10MB
MAX_LINE_LENGTH = 10000  # Characters per line


class FileOperationsSandbox:
    """Sandbox for file operations to prevent path traversal attacks.

    Validates that all file paths are within allowed root directories,
    preventing unauthorized access to files outside the sandbox.

    Example:
        >>> sandbox = FileOperationsSandbox([Path("/home/user/project")])
        >>> sandbox.validate_path("/home/user/project/src/main.py")  # OK
        >>> sandbox.validate_path("/etc/passwd")  # Raises PermissionError
    """

    def __init__(self, allowed_roots: list[Path] | None = None) -> None:
        """Initialize the sandbox.

        Args:
            allowed_roots: List of allowed root directories. Defaults to [cwd()].
        """
        self.allowed_roots = allowed_roots or [Path.cwd()]

    def validate_path(self, path: str | Path) -> Path:
        """Validate a path is within allowed directories.

        Args:
            path: Path to validate (can be relative or absolute).

        Returns:
            Resolved absolute path if valid.

        Raises:
            PermissionError: If path is outside allowed directories.
        """
        resolved = Path(path).resolve()
        for root in self.allowed_roots:
            try:
                resolved.relative_to(root.resolve())
                return resolved
            except ValueError:
                continue
        raise PermissionError(
            f"Path '{resolved}' is outside allowed directories: "
            f"{[str(r) for r in self.allowed_roots]}"
        )


# Global sandbox instance - can be reconfigured at runtime
_file_sandbox = FileOperationsSandbox()


def should_ignore(path: Path, patterns: tuple[str, ...] = IGNORE_PATTERNS) -> bool:
    """Check if a path should be ignored based on patterns.

    Args:
        path: Path to check.
        patterns: Tuple of patterns to match against.

    Returns:
        True if the path matches any ignore pattern.
    """
    name = path.name
    path_str = str(path)

    for pattern in patterns:
        # Check exact name match
        if name == pattern:
            return True
        # Check glob pattern on name
        if fnmatch.fnmatch(name, pattern):
            return True
        # Check if pattern appears in path
        if pattern in path_str:
            return True

    return False


def estimate_tokens(text: str) -> int:
    """Estimate token count for text.

    Uses character-based estimation (4 chars per token average).

    Args:
        text: Text to estimate.

    Returns:
        Estimated token count.
    """
    # Simple character-based estimation
    # Average 4 characters per token for English text
    return len(text) // 4


# =============================================================================
# List Files Tool
# =============================================================================


class ListFilesInput(BaseModel):
    """Input for list_files tool.

    Attributes:
        directory: Path to the directory to list.
        recursive: Whether to recursively list subdirectories.
    """

    directory: str = Field(
        default=".",
        description="Path to the directory to list",
    )
    recursive: bool = Field(
        default=True,
        description="Whether to recursively list subdirectories",
    )


class FileEntry(BaseModel):
    """Information about a file or directory.

    Attributes:
        path: Relative path to the file.
        type: Type of entry ('file' or 'directory').
        size: File size in bytes (None for directories).
        depth: Depth from the root directory.
    """

    path: str = Field(description="Relative path")
    type: str = Field(description="Entry type (file/directory)")
    size: int | None = Field(default=None, description="Size in bytes")
    depth: int = Field(default=0, description="Directory depth")


class ListFilesOutput(BaseModel):
    """Output from list_files tool.

    Attributes:
        content: String representation of the directory listing.
        error: Error message if listing failed.
    """

    content: str = Field(default="", description="Directory listing")
    error: str | None = Field(default=None, description="Error message")


class ListFilesTool(BaseTool[ListFilesInput, ListFilesOutput]):
    """Tool for listing directory contents.

    Features:
    - Recursive directory traversal
    - Intelligent ignore patterns
    - Token-safe output limits
    - Home directory detection
    """

    def __init__(self, allow_recursion: bool = True) -> None:
        """Initialize the tool.

        Args:
            allow_recursion: Whether to allow recursive listing.
        """
        super().__init__(
            ToolConfig(
                name="list_files",
                description=(
                    "List files and directories with intelligent filtering. "
                    "Automatically ignores common build artifacts and cache directories."
                ),
            )
        )
        self._allow_recursion = allow_recursion

    async def execute(
        self,
        input_data: ListFilesInput,
        context: RunContext | None = None,
    ) -> ListFilesOutput:
        """Execute the list_files operation.

        Args:
            input_data: Input parameters.
            context: Optional run context.

        Returns:
            ListFilesOutput with directory listing.
        """
        try:
            directory = Path(input_data.directory).resolve()

            if not directory.exists():
                return ListFilesOutput(
                    error=f"Directory does not exist: {directory}"
                )

            if not directory.is_dir():
                return ListFilesOutput(
                    error=f"Path is not a directory: {directory}"
                )

            # Check if this is a home directory (limit recursion)
            home = Path.home()
            is_home_dir = directory == home
            recursive = input_data.recursive and self._allow_recursion

            # Disable recursion for home directories without project indicators
            if is_home_dir and recursive:
                project_indicators = {
                    ".git",
                    "package.json",
                    "pyproject.toml",
                    "Cargo.toml",
                    "go.mod",
                }
                has_project = any((directory / ind).exists() for ind in project_indicators)
                if not has_project:
                    recursive = False

            entries = self._collect_entries(directory, recursive)

            # Format output
            output_lines = [
                f"DIRECTORY LISTING: {directory} (recursive={recursive})"
            ]

            for entry in entries:
                size_str = f" ({self._format_size(entry.size)})" if entry.size else "/"
                indent = "  " * entry.depth
                output_lines.append(f"{indent}{entry.path}{size_str}")

            output_lines.append(
                f"\nSummary: {sum(1 for e in entries if e.type == 'directory')} directories, "
                f"{sum(1 for e in entries if e.type == 'file')} files"
            )

            return ListFilesOutput(content="\n".join(output_lines))

        except PermissionError as e:
            return ListFilesOutput(error=f"Permission denied: {e}")
        except OSError as e:
            return ListFilesOutput(error=f"OS error: {e}")

    def _collect_entries(
        self,
        root: Path,
        recursive: bool,
        base_depth: int = 0,
    ) -> list[FileEntry]:
        """Collect file entries from a directory.

        Args:
            root: Root directory.
            recursive: Whether to recurse.
            base_depth: Starting depth.

        Returns:
            List of FileEntry objects.
        """
        entries: list[FileEntry] = []

        try:
            items = sorted(root.iterdir(), key=lambda p: (not p.is_dir(), p.name.lower()))
        except PermissionError:
            return entries

        for item in items:
            if should_ignore(item):
                continue

            if len(entries) >= MAX_LIST_FILES:
                break

            rel_path = item.name

            if item.is_dir():
                entries.append(
                    FileEntry(
                        path=rel_path,
                        type="directory",
                        depth=base_depth,
                    )
                )
                if recursive:
                    sub_entries = self._collect_entries(
                        item, recursive, base_depth + 1
                    )
                    entries.extend(sub_entries)
            elif item.is_file():
                try:
                    size = item.stat().st_size
                except OSError:
                    size = None
                entries.append(
                    FileEntry(
                        path=rel_path,
                        type="file",
                        size=size,
                        depth=base_depth,
                    )
                )

        return entries

    def _format_size(self, size: int | None) -> str:
        """Format file size in human-readable form."""
        if size is None:
            return "unknown"

        for unit in ["", "KB", "MB", "GB"]:
            if abs(size) < 1024.0:
                return f"{size:.1f} {unit}".strip()
            size = int(size / 1024)
        return f"{size:.1f} TB"

    def get_schema(self) -> dict[str, Any]:
        """Get JSON schema for this tool."""
        return {
            "name": self.name,
            "description": self.description,
            "parameters": ListFilesInput.model_json_schema(),
        }


# =============================================================================
# Read File Tool
# =============================================================================


class ReadFileInput(BaseModel):
    """Input for read_file tool.

    Attributes:
        file_path: Path to the file to read.
        start_line: Starting line number (1-based).
        num_lines: Number of lines to read from start_line.
    """

    file_path: str = Field(
        description="Path to the file to read",
    )
    start_line: int | None = Field(
        default=None,
        ge=1,
        description="Starting line number (1-based)",
    )
    num_lines: int | None = Field(
        default=None,
        ge=1,
        description="Number of lines to read",
    )


class ReadFileOutput(BaseModel):
    """Output from read_file tool.

    Attributes:
        content: File content or error message.
        num_tokens: Estimated token count.
        error: Error message if reading failed.
    """

    content: str | None = Field(default=None, description="File content")
    num_tokens: int = Field(default=0, description="Estimated token count")
    error: str | None = Field(default=None, description="Error message")


class ReadFileTool(BaseTool[ReadFileInput, ReadFileOutput]):
    """Tool for reading file contents.

    Features:
    - Token estimation for safety
    - Line-range selection for large files
    - Encoding error handling
    """

    MAX_TOKENS: ClassVar[int] = 10000

    def __init__(self) -> None:
        """Initialize the tool."""
        super().__init__(
            ToolConfig(
                name="read_file",
                description=(
                    "Read file contents with optional line-range selection. "
                    "Includes token estimation for safety."
                ),
            )
        )

    async def execute(
        self,
        input_data: ReadFileInput,
        context: RunContext | None = None,
    ) -> ReadFileOutput:
        """Execute the read_file operation.

        Args:
            input_data: Input parameters.
            context: Optional run context.

        Returns:
            ReadFileOutput with file content.
        """
        if not input_data.file_path:
            return ReadFileOutput(error="File path cannot be empty")

        try:
            file_path = Path(input_data.file_path).resolve()

            if not file_path.exists():
                return ReadFileOutput(error=f"File does not exist: {file_path}")

            if not file_path.is_file():
                return ReadFileOutput(error=f"Path is not a file: {file_path}")

            # Check file size
            size = file_path.stat().st_size
            if size > MAX_FILE_SIZE_BYTES:
                return ReadFileOutput(
                    error=f"File too large: {size} bytes (max {MAX_FILE_SIZE_BYTES})"
                )

            # Read file with encoding fallback
            content = self._read_with_fallback(file_path)

            # Apply line range if specified
            if input_data.start_line is not None:
                if input_data.num_lines is None:
                    return ReadFileOutput(
                        error="num_lines must be specified when start_line is provided"
                    )
                lines = content.splitlines(keepends=True)
                start_idx = input_data.start_line - 1  # Convert to 0-based
                end_idx = start_idx + input_data.num_lines
                content = "".join(lines[start_idx:end_idx])

            # Estimate tokens
            num_tokens = estimate_tokens(content)

            # Warn if content is large
            if num_tokens > self.MAX_TOKENS:
                return ReadFileOutput(
                    content=content[:40000],  # ~10k tokens
                    num_tokens=self.MAX_TOKENS,
                    error=f"Content truncated: {num_tokens} estimated tokens exceeds limit",
                )

            return ReadFileOutput(content=content, num_tokens=num_tokens)

        except PermissionError:
            return ReadFileOutput(error=f"Permission denied: {input_data.file_path}")
        except OSError as e:
            return ReadFileOutput(error=f"Error reading file: {e}")

    def _read_with_fallback(self, path: Path) -> str:
        """Read file with encoding fallback.

        Args:
            path: File path.

        Returns:
            File content.
        """
        encodings = ["utf-8", "utf-8-sig", "latin-1", "cp1252"]

        for encoding in encodings:
            try:
                return path.read_text(encoding=encoding)
            except UnicodeDecodeError:
                continue

        # Last resort: read as bytes and decode with replacement
        return path.read_bytes().decode("utf-8", errors="replace")

    def get_schema(self) -> dict[str, Any]:
        """Get JSON schema for this tool."""
        return {
            "name": self.name,
            "description": self.description,
            "parameters": ReadFileInput.model_json_schema(),
        }


# =============================================================================
# Grep Tool
# =============================================================================


class GrepInput(BaseModel):
    """Input for grep tool.

    Attributes:
        search_string: Text pattern to search for.
        directory: Root directory for the search.
    """

    search_string: str = Field(
        description="Text pattern to search for (supports ripgrep flags)",
    )
    directory: str = Field(
        default=".",
        description="Root directory for the search",
    )


class MatchInfo(BaseModel):
    """Information about a search match.

    Attributes:
        file_path: Path to the file containing the match.
        line_number: Line number of the match (1-based).
        line_content: Content of the matching line.
    """

    file_path: str | None = Field(default=None, description="File path")
    line_number: int | None = Field(default=None, description="Line number")
    line_content: str | None = Field(default=None, description="Line content")


class GrepOutput(BaseModel):
    """Output from grep tool.

    Attributes:
        matches: List of matches found.
        error: Error message if search failed.
    """

    matches: list[MatchInfo] = Field(
        default_factory=list,
        description="List of matches",
    )
    error: str | None = Field(default=None, description="Error message")


class GrepTool(BaseTool[GrepInput, GrepOutput]):
    """Tool for searching text patterns across files.

    Uses ripgrep (rg) for high-performance searching.

    Features:
    - Regex pattern support
    - Automatic ignore patterns
    - Max 200 matches for safety
    - Line content truncation
    """

    def __init__(self) -> None:
        """Initialize the tool."""
        super().__init__(
            ToolConfig(
                name="grep",
                description=(
                    "Search for text patterns across files using ripgrep. "
                    "Supports regex and respects ignore patterns."
                ),
            )
        )

    async def execute(
        self,
        input_data: GrepInput,
        context: RunContext | None = None,
    ) -> GrepOutput:
        """Execute the grep operation.

        Args:
            input_data: Input parameters.
            context: Optional run context.

        Returns:
            GrepOutput with search matches.
        """
        if not input_data.search_string.strip():
            return GrepOutput(error="Search string cannot be empty")

        try:
            # Validate path is within sandbox
            try:
                directory = _file_sandbox.validate_path(input_data.directory)
            except PermissionError as e:
                return GrepOutput(error=str(e))

            if not directory.exists():
                return GrepOutput(error=f"Directory does not exist: {directory}")

            # Try ripgrep first, fall back to Python implementation
            try:
                return await self._ripgrep_search(
                    input_data.search_string, directory
                )
            except FileNotFoundError:
                return await self._python_search(
                    input_data.search_string, directory
                )

        except Exception as e:
            return GrepOutput(error=f"Search error: {e}")

    async def _ripgrep_search(
        self,
        pattern: str,
        directory: Path,
    ) -> GrepOutput:
        """Search using ripgrep.

        Args:
            pattern: Search pattern.
            directory: Directory to search.

        Returns:
            GrepOutput with matches.
        """
        cmd = [
            "rg",
            "--line-number",
            "--no-heading",
            "--with-filename",
            "--max-count",
            str(MAX_GREP_MATCHES),
            pattern,
            str(directory),
        ]

        proc = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=60,
        )

        matches: list[MatchInfo] = []

        for line in proc.stdout.splitlines():
            if len(matches) >= MAX_GREP_MATCHES:
                break

            # Parse ripgrep output: file:line:content
            parts = line.split(":", 2)
            if len(parts) >= 3:
                file_path, line_num, content = parts[0], parts[1], parts[2]
                try:
                    matches.append(
                        MatchInfo(
                            file_path=str(Path(file_path).resolve()),
                            line_number=int(line_num),
                            line_content=content[:MAX_LINE_LENGTH],
                        )
                    )
                except ValueError:
                    continue

        return GrepOutput(matches=matches)

    async def _python_search(
        self,
        pattern: str,
        directory: Path,
    ) -> GrepOutput:
        """Fallback Python-based search.

        Args:
            pattern: Search pattern.
            directory: Directory to search.

        Returns:
            GrepOutput with matches.
        """
        matches: list[MatchInfo] = []

        try:
            regex = re.compile(pattern)
        except re.error as e:
            return GrepOutput(error=f"Invalid regex pattern: {e}")

        for file_path in directory.rglob("*"):
            if len(matches) >= MAX_GREP_MATCHES:
                break

            if not file_path.is_file():
                continue

            if should_ignore(file_path):
                continue

            try:
                content = file_path.read_text(encoding="utf-8", errors="ignore")
                for line_num, line in enumerate(content.splitlines(), 1):
                    if len(matches) >= MAX_GREP_MATCHES:
                        break
                    if regex.search(line):
                        matches.append(
                            MatchInfo(
                                file_path=str(file_path.resolve()),
                                line_number=line_num,
                                line_content=line[:MAX_LINE_LENGTH],
                            )
                        )
            except (OSError, UnicodeDecodeError):
                continue

        return GrepOutput(matches=matches)

    def get_schema(self) -> dict[str, Any]:
        """Get JSON schema for this tool."""
        return {
            "name": self.name,
            "description": self.description,
            "parameters": GrepInput.model_json_schema(),
        }


# =============================================================================
# Edit File Tool
# =============================================================================


class Replacement(BaseModel):
    """A single text replacement.

    Attributes:
        old_str: Text to find and replace.
        new_str: Replacement text.
    """

    old_str: str = Field(description="Text to find")
    new_str: str = Field(description="Replacement text")


class ContentPayload(BaseModel):
    """Payload for full content replacement.

    Attributes:
        file_path: Path to the file.
        content: Full file content to write.
        overwrite: Whether to overwrite existing files.
    """

    file_path: str = Field(description="Path to file")
    content: str = Field(description="Full file content")
    overwrite: bool = Field(
        default=False,
        description="Whether to overwrite existing files",
    )


class ReplacementsPayload(BaseModel):
    """Payload for targeted text replacements.

    Attributes:
        file_path: Path to the file.
        replacements: List of text replacements.
    """

    file_path: str = Field(description="Path to file")
    replacements: list[Replacement] = Field(description="Text replacements")


class DeleteSnippetPayload(BaseModel):
    """Payload for snippet deletion.

    Attributes:
        file_path: Path to the file.
        delete_snippet: Text snippet to remove.
    """

    file_path: str = Field(description="Path to file")
    delete_snippet: str = Field(description="Text to delete")


EditPayload = ContentPayload | ReplacementsPayload | DeleteSnippetPayload


class EditFileOutput(BaseModel):
    """Output from edit_file tool.

    Attributes:
        success: Whether the operation succeeded.
        path: Absolute path to the file.
        message: Description of changes.
        changed: Whether file content was modified.
        diff: Unified diff showing changes.
        error: Error message if operation failed.
    """

    success: bool = Field(default=False, description="Operation success")
    path: str = Field(default="", description="Absolute file path")
    message: str = Field(default="", description="Result message")
    changed: bool = Field(default=False, description="Whether content changed")
    diff: str | None = Field(default=None, description="Unified diff")
    error: str | None = Field(default=None, description="Error message")


class EditFileTool(BaseTool[EditPayload, EditFileOutput]):
    """Tool for editing files.

    Supports three editing modes:
    1. Full content replacement
    2. Targeted text replacements
    3. Snippet deletion

    Features:
    - Diff generation
    - Safe mode (no overwrite by default)
    - Automatic parent directory creation
    """

    def __init__(self) -> None:
        """Initialize the tool."""
        super().__init__(
            ToolConfig(
                name="edit_file",
                description=(
                    "Edit files by writing content, making replacements, or deleting snippets. "
                    "Generates diffs to show changes."
                ),
                requires_confirmation=True,
            )
        )

    async def execute(
        self,
        input_data: EditPayload,
        context: RunContext | None = None,
    ) -> EditFileOutput:
        """Execute the edit_file operation.

        Args:
            input_data: Input payload (one of three types).
            context: Optional run context.

        Returns:
            EditFileOutput with result.
        """
        try:
            # Validate path is within sandbox first
            try:
                _file_sandbox.validate_path(input_data.file_path)
            except PermissionError as e:
                return EditFileOutput(error=str(e))

            if isinstance(input_data, ContentPayload):
                return await self._write_content(input_data)
            elif isinstance(input_data, ReplacementsPayload):
                return await self._apply_replacements(input_data)
            elif isinstance(input_data, DeleteSnippetPayload):
                return await self._delete_snippet(input_data)
            else:
                return EditFileOutput(error="Unknown payload type")

        except PermissionError:
            return EditFileOutput(
                error=f"Permission denied: {input_data.file_path}"
            )
        except OSError as e:
            return EditFileOutput(error=f"OS error: {e}")

    async def _write_content(
        self,
        payload: ContentPayload,
    ) -> EditFileOutput:
        """Write full content to a file.

        Args:
            payload: Content payload.

        Returns:
            EditFileOutput with result.
        """
        file_path = Path(payload.file_path).resolve()

        # Check if file exists and overwrite is not allowed
        if file_path.exists() and not payload.overwrite:
            return EditFileOutput(
                success=False,
                path=str(file_path),
                error="File exists and overwrite=False",
            )

        # Read original content for diff
        original = ""
        if file_path.exists():
            try:
                original = file_path.read_text(encoding="utf-8")
            except UnicodeDecodeError:
                original = file_path.read_bytes().decode("utf-8", errors="replace")

        # Check if content actually changed
        if original == payload.content:
            return EditFileOutput(
                success=True,
                path=str(file_path),
                message="No changes needed",
                changed=False,
            )

        # Create parent directories if needed
        file_path.parent.mkdir(parents=True, exist_ok=True)

        # Write the file
        file_path.write_text(payload.content, encoding="utf-8")

        # Generate diff
        diff = self._generate_diff(original, payload.content, str(file_path))

        action = "Created" if not original else "Updated"
        return EditFileOutput(
            success=True,
            path=str(file_path),
            message=f"{action} {file_path}",
            changed=True,
            diff=diff,
        )

    async def _apply_replacements(
        self,
        payload: ReplacementsPayload,
    ) -> EditFileOutput:
        """Apply text replacements to a file.

        Args:
            payload: Replacements payload.

        Returns:
            EditFileOutput with result.
        """
        file_path = Path(payload.file_path).resolve()

        if not file_path.exists():
            return EditFileOutput(
                error=f"File does not exist: {file_path}"
            )

        # Read original content
        try:
            original = file_path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            original = file_path.read_bytes().decode("utf-8", errors="replace")

        # Apply replacements
        content = original
        for replacement in payload.replacements:
            if replacement.old_str not in content:
                return EditFileOutput(
                    error=f"Text not found: {replacement.old_str[:100]}..."
                )
            content = content.replace(
                replacement.old_str,
                replacement.new_str,
                1,  # Only replace first occurrence
            )

        # Check if content actually changed
        if original == content:
            return EditFileOutput(
                success=True,
                path=str(file_path),
                message="No changes needed",
                changed=False,
            )

        # Write the file
        file_path.write_text(content, encoding="utf-8")

        # Generate diff
        diff = self._generate_diff(original, content, str(file_path))

        return EditFileOutput(
            success=True,
            path=str(file_path),
            message=f"Applied {len(payload.replacements)} replacement(s)",
            changed=True,
            diff=diff,
        )

    async def _delete_snippet(
        self,
        payload: DeleteSnippetPayload,
    ) -> EditFileOutput:
        """Delete a snippet from a file.

        Args:
            payload: Delete snippet payload.

        Returns:
            EditFileOutput with result.
        """
        file_path = Path(payload.file_path).resolve()

        if not file_path.exists():
            return EditFileOutput(
                error=f"File does not exist: {file_path}"
            )

        # Read original content
        try:
            original = file_path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            original = file_path.read_bytes().decode("utf-8", errors="replace")

        # Delete the snippet
        if payload.delete_snippet not in original:
            return EditFileOutput(
                error=f"Snippet not found: {payload.delete_snippet[:100]}..."
            )

        content = original.replace(payload.delete_snippet, "", 1)

        # Check if content actually changed
        if original == content:
            return EditFileOutput(
                success=True,
                path=str(file_path),
                message="No changes needed",
                changed=False,
            )

        # Write the file
        file_path.write_text(content, encoding="utf-8")

        # Generate diff
        diff = self._generate_diff(original, content, str(file_path))

        return EditFileOutput(
            success=True,
            path=str(file_path),
            message="Deleted snippet",
            changed=True,
            diff=diff,
        )

    def _generate_diff(
        self,
        original: str,
        modified: str,
        path: str,
    ) -> str:
        """Generate a unified diff between two strings.

        Args:
            original: Original content.
            modified: Modified content.
            path: File path for diff header.

        Returns:
            Unified diff string.
        """
        original_lines = original.splitlines(keepends=True)
        modified_lines = modified.splitlines(keepends=True)

        diff = difflib.unified_diff(
            original_lines,
            modified_lines,
            fromfile=f"a/{path}",
            tofile=f"b/{path}",
            lineterm="",
        )

        return "".join(diff)

    def get_schema(self) -> dict[str, Any]:
        """Get JSON schema for this tool."""
        # Return a union schema for the three payload types
        return {
            "name": self.name,
            "description": self.description,
            "parameters": {
                "type": "object",
                "oneOf": [
                    ContentPayload.model_json_schema(),
                    ReplacementsPayload.model_json_schema(),
                    DeleteSnippetPayload.model_json_schema(),
                ],
            },
        }


# =============================================================================
# Delete File Tool
# =============================================================================


class DeleteFileInput(BaseModel):
    """Input for delete_file tool.

    Attributes:
        file_path: Path to the file to delete.
    """

    file_path: str = Field(description="Path to the file to delete")


class DeleteFileOutput(BaseModel):
    """Output from delete_file tool.

    Attributes:
        success: Whether the deletion succeeded.
        path: Absolute path to the deleted file.
        message: Result description.
        changed: Whether a file was actually removed.
        error: Error message if deletion failed.
    """

    success: bool = Field(default=False, description="Operation success")
    path: str = Field(default="", description="Absolute file path")
    message: str = Field(default="", description="Result message")
    changed: bool = Field(default=False, description="Whether file was removed")
    error: str | None = Field(default=None, description="Error message")


class DeleteFileTool(BaseTool[DeleteFileInput, DeleteFileOutput]):
    """Tool for deleting files.

    Features:
    - Safe deletion with existence check
    - Comprehensive logging
    - Error handling
    """

    def __init__(self) -> None:
        """Initialize the tool."""
        super().__init__(
            ToolConfig(
                name="delete_file",
                description="Safely delete a file from the filesystem.",
                requires_confirmation=True,
            )
        )

    async def execute(
        self,
        input_data: DeleteFileInput,
        context: RunContext | None = None,
    ) -> DeleteFileOutput:
        """Execute the delete_file operation.

        Args:
            input_data: Input parameters.
            context: Optional run context.

        Returns:
            DeleteFileOutput with result.
        """
        if not input_data.file_path:
            return DeleteFileOutput(error="File path cannot be empty")

        try:
            # Validate path is within sandbox
            try:
                file_path = _file_sandbox.validate_path(input_data.file_path)
            except PermissionError as e:
                return DeleteFileOutput(error=str(e))

            if not file_path.exists():
                return DeleteFileOutput(
                    success=True,
                    path=str(file_path),
                    message="File does not exist",
                    changed=False,
                )

            if not file_path.is_file():
                return DeleteFileOutput(
                    error=f"Path is not a regular file: {file_path}"
                )

            # Delete the file
            file_path.unlink()

            return DeleteFileOutput(
                success=True,
                path=str(file_path),
                message=f"Deleted {file_path}",
                changed=True,
            )

        except PermissionError:
            return DeleteFileOutput(
                error=f"Permission denied: {input_data.file_path}"
            )
        except OSError as e:
            return DeleteFileOutput(error=f"Delete error: {e}")

    def get_schema(self) -> dict[str, Any]:
        """Get JSON schema for this tool."""
        return {
            "name": self.name,
            "description": self.description,
            "parameters": DeleteFileInput.model_json_schema(),
        }


__all__ = [
    # Sandbox
    "FileOperationsSandbox",
    # Input/Output models
    "ContentPayload",
    "DeleteFileInput",
    "DeleteFileOutput",
    "DeleteSnippetPayload",
    "EditFileOutput",
    "EditPayload",
    "FileEntry",
    "GrepInput",
    "GrepOutput",
    "ListFilesInput",
    "ListFilesOutput",
    "MatchInfo",
    "ReadFileInput",
    "ReadFileOutput",
    "Replacement",
    "ReplacementsPayload",
    # Tools
    "DeleteFileTool",
    "EditFileTool",
    "GrepTool",
    "ListFilesTool",
    "ReadFileTool",
    # Utilities
    "IGNORE_PATTERNS",
    "estimate_tokens",
    "should_ignore",
]
