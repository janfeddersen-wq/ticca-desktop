//! Skills Agent - Python-based skill execution
//!
//! This agent dynamically discovers and uses Python-based skills stored in
//! the skills directory. Each skill provides specialized capabilities that
//! extend the agent's functionality.

use std::path::PathBuf;

use anyhow::{Context, Result};
use tracing::{debug, info};

use super::PromptBlocks;
use super::base::{Agent, AgentType};
use crate::config::paths::{get_skills_dir, get_tools_dir, get_venvs_dir};
use crate::external_tools::catalog::get_tool_definition;
use crate::external_tools::manifest::load_manifest;
use crate::external_tools::types::{ExternalToolId, Platform};
use crate::python::create_venv;
use crate::skills::{SkillMetadata, discover_skills};
use crate::tools::spec::tool_specs_for_names;

/// Information about an installed external tool.
#[derive(Debug, Clone)]
pub struct InstalledTool {
    pub name: String,
    pub executable_path: PathBuf,
}

/// Discovers installed external tools and returns their paths.
fn discover_installed_tools() -> Vec<InstalledTool> {
    let platform = match Platform::detect() {
        Some(p) => p,
        None => return Vec::new(),
    };

    let tools_dir = match get_tools_dir() {
        Ok(dir) => dir,
        Err(_) => return Vec::new(),
    };

    let manifest = match load_manifest() {
        Ok(m) => m,
        Err(_) => return Vec::new(),
    };

    let mut installed = Vec::new();

    for tool_id in ExternalToolId::all() {
        if !manifest.is_installed(*tool_id) {
            continue;
        }

        let def = get_tool_definition(*tool_id);
        let exec_relpath = def.get_executable_path(platform);
        let tool_dir = tools_dir.join(tool_id.as_str());
        let exec_path = tool_dir.join(exec_relpath);

        if exec_path.exists() {
            installed.push(InstalledTool {
                name: def.display_name.to_string(),
                executable_path: exec_path,
            });
        }
    }

    installed
}

/// Skills Agent - executes Python-based skills for specialized tasks.
///
/// This agent discovers skills from the skills directory at runtime and
/// provides access to them through the system prompt. Each skill is a
/// Python-based module with a SKILL.md file describing its capabilities.
pub struct SkillsAgent {
    /// Discovered skill metadata (populated at runtime)
    skills: Vec<SkillMetadata>,
    /// Path to the shared skills venv
    venv_path: PathBuf,
    /// Path to the skills directory
    skills_dir: PathBuf,
    /// Installed external tools (pandoc, node, libreoffice)
    installed_tools: Vec<InstalledTool>,
}

impl SkillsAgent {
    /// Creates a new SkillsAgent by discovering available skills.
    ///
    /// This should be called at app startup after skills are extracted.
    /// The agent will:
    /// 1. Get paths from the config module
    /// 2. Discover skills using `discover_skills()`
    /// 3. Create a shared venv for skills if it doesn't exist
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The skills or venvs directory cannot be determined
    /// - Skill discovery fails
    /// - The venv cannot be created
    pub fn new() -> Result<Self> {
        let skills_dir = get_skills_dir().context("Failed to get skills directory")?;
        let venv_path = get_venvs_dir()
            .context("Failed to get venvs directory")?
            .join("skills-venv");

        // Discover available skills
        let skills = discover_skills().context("Failed to discover skills")?;
        info!("Discovered {} skills", skills.len());

        for skill in &skills {
            debug!("Found skill: {} at {}", skill.name, skill.path.display());
        }

        // Discover installed external tools
        let installed_tools = discover_installed_tools();
        info!(
            "Discovered {} installed external tools",
            installed_tools.len()
        );

        for tool in &installed_tools {
            debug!(
                "Found tool: {} at {}",
                tool.name,
                tool.executable_path.display()
            );
        }

        // Create the venv if it doesn't exist
        if !venv_path.exists() {
            info!("Creating skills venv at {}", venv_path.display());
            create_venv(&venv_path).context("Failed to create skills venv")?;
        } else {
            debug!("Skills venv already exists at {}", venv_path.display());
        }

        Ok(Self {
            skills,
            venv_path,
            skills_dir,
            installed_tools,
        })
    }

    /// Creates a SkillsAgent with default/empty state.
    ///
    /// This is useful when skill discovery fails but we still want to
    /// create an agent instance. The agent will have no skills available.
    pub fn empty() -> Self {
        Self {
            skills: Vec::new(),
            venv_path: PathBuf::new(),
            skills_dir: PathBuf::new(),
            installed_tools: Vec::new(),
        }
    }

    /// Returns the combined description from all discovered skills.
    ///
    /// Format: "Agent that provides the following skills: skill1 - desc1, skill2 - desc2, ..."
    pub fn combined_description(&self) -> String {
        if self.skills.is_empty() {
            return "Agent for executing Python-based skills. No skills currently available."
                .to_string();
        }

        let skill_list: Vec<String> = self
            .skills
            .iter()
            .map(|s| format!("{} - {}", s.name, s.description))
            .collect();

        format!(
            "Agent that provides the following skills: {}",
            skill_list.join(", ")
        )
    }

    /// Returns whether any skills were discovered.
    pub fn has_skills(&self) -> bool {
        !self.skills.is_empty()
    }

    /// Get the discovered skills.
    pub fn skills(&self) -> &[SkillMetadata] {
        &self.skills
    }

    /// Get the venv path.
    pub fn venv_path(&self) -> &PathBuf {
        &self.venv_path
    }

    /// Get the skills directory.
    pub fn skills_dir(&self) -> &PathBuf {
        &self.skills_dir
    }

    /// Get the Python interpreter path for this agent's venv.
    fn python_path(&self) -> PathBuf {
        #[cfg(windows)]
        {
            self.venv_path.join("Scripts").join("python.exe")
        }
        #[cfg(not(windows))]
        {
            self.venv_path.join("bin").join("python")
        }
    }

    /// Builds the skills section of the system prompt.
    fn build_skills_section(&self) -> String {
        if self.skills.is_empty() {
            return "## Available Skills\n\nNo skills are currently available.\n".to_string();
        }

        let mut out = String::from("## Available Skills\n\n");
        out.push_str("The following Python-based skills are available. Before using any skill, ");
        out.push_str("READ its SKILL.md file to understand usage, parameters, and examples.\n\n");

        for skill in &self.skills {
            out.push_str(&format!("### {}\n", skill.name));
            out.push_str(&format!("**Description:** {}\n", skill.description));
            out.push_str(&format!("**License:** {}\n", skill.license));
            out.push_str(&format!("**Path:** {}\n", skill.path.display()));
            out.push_str(&format!(
                "**Documentation:** {} (READ THIS BEFORE USING)\n",
                skill.skill_md_path.display()
            ));
            out.push('\n');
        }

        out
    }

    /// Builds the external tools section of the system prompt.
    fn build_tools_section(&self) -> String {
        if self.installed_tools.is_empty() {
            return "## External Tools\n\nNo external tools are currently installed.\n".to_string();
        }

        let mut out = String::from("## External Tools\n\n");
        out.push_str("The following external tools are installed and available for use:\n\n");

        for tool in &self.installed_tools {
            out.push_str(&format!(
                "- **{}**: `{}`\n",
                tool.name,
                tool.executable_path.display()
            ));
        }

        out.push_str("\nYou can invoke these tools directly using `execute_shell`.\n");
        out
    }

    /// Get the installed external tools.
    pub fn installed_tools(&self) -> &[InstalledTool] {
        &self.installed_tools
    }
}

impl Agent for SkillsAgent {
    fn agent_type(&self) -> AgentType {
        AgentType::Skills
    }

    fn display_name(&self) -> &'static str {
        "Skills Agent"
    }

    fn description(&self) -> String {
        self.combined_description()
    }

    fn available_tools(&self) -> Vec<&'static str> {
        // Same tools as CodingAgent - full access to file and shell tools
        vec![
            "todo_read",
            "todo_write",
            "todo_list",
            "list_files",
            "read_file",
            "grep",
            "list_agents",
            "invoke_agent",
            "edit_file",
            "delete_file",
            "write_file",
            "execute_shell",
            "list_processes",
            "read_process_output",
            "kill_process",
        ]
    }

    fn system_prompt(&self) -> String {
        let tool_specs = tool_specs_for_names(&self.available_tools());
        let tool_docs = PromptBlocks::tool_docs(&tool_specs);

        let skills_section = self.build_skills_section();
        let tools_section = self.build_tools_section();
        let python_path = self.python_path();

        let workflow = r#"## Core Workflow

You must follow this iterative, three-step cycle for every action:

1. **Reason**: Articulate your immediate goal, the specific skill or tool you will use, and the expected outcome.
2. **Execute**: Invoke a single tool (`execute_shell` for a skill, or another file tool) to perform the planned action.
3. **Validate**: Analyze the output to confirm success or failure, then report the result and determine the next step."#;

        let directives = format!(
            r#"## Critical Directives

1. **Use the Venv**: Executing Python scripts with any interpreter other than `{}` is strictly prohibited.
2. **Consult Documentation**: You MUST read a skill's `SKILL.md` file to understand its API, arguments, and requirements before using it.
3. **Action is Mandatory**: You MUST use tools to accomplish tasks. Do not describe what should be done; do it.
4. **Autonomy is Key**: Continue the Reason-Execute-Validate cycle autonomously until the task is complete.
5. **Adhere to File Size Limits**: No file may exceed 600 lines. If a file you are modifying approaches this limit, you MUST refactor it.
6. **Update To-Do List**: You must use `todo_write` or `todo_list` to mark tasks as complete upon finishing your work."#,
            python_path.display()
        );

        format!(
            r#"You are a specialist Skills Agent. You are equipped with a suite of Python-based skills and external tools to perform complex, specialized tasks beyond standard coding. You must operate with precision, leveraging the correct tools as required.

## Python Environment

The Python virtual environment for skills is located at:
`{venv_path}`

**CRITICAL:** ALL Python executions MUST use the designated interpreter to ensure dependency resolution:
`{python_path}`

Example shell command:
```bash
{python_path} /path/to/your/script.py --arg1 value
```

{tools_section}
{skills_section}
{tool_docs}
{workflow}

{directives}"#,
            venv_path = self.venv_path.display(),
            python_path = python_path.display(),
            tools_section = tools_section,
            skills_section = skills_section,
            tool_docs = tool_docs,
            workflow = workflow,
            directives = directives,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skills_agent_empty() {
        let agent = SkillsAgent::empty();

        assert!(!agent.has_skills());
        assert!(agent.skills().is_empty());
        assert_eq!(agent.agent_type(), AgentType::Skills);
        assert_eq!(agent.display_name(), "Skills Agent");
    }

    #[test]
    fn test_combined_description_empty() {
        let agent = SkillsAgent::empty();
        let desc = agent.combined_description();

        assert!(desc.contains("No skills currently available"));
    }

    #[test]
    fn test_combined_description_with_skills() {
        let agent = SkillsAgent {
            skills: vec![
                SkillMetadata {
                    name: "docx".to_string(),
                    description: "Document creation".to_string(),
                    license: "MIT".to_string(),
                    path: PathBuf::from("/skills/docx"),
                    skill_md_path: PathBuf::from("/skills/docx/SKILL.md"),
                },
                SkillMetadata {
                    name: "browser".to_string(),
                    description: "Web automation".to_string(),
                    license: "Apache-2.0".to_string(),
                    path: PathBuf::from("/skills/browser"),
                    skill_md_path: PathBuf::from("/skills/browser/SKILL.md"),
                },
            ],
            venv_path: PathBuf::from("/venvs/skills-venv"),
            skills_dir: PathBuf::from("/skills"),
            installed_tools: Vec::new(),
        };

        let desc = agent.combined_description();
        assert!(desc.contains("docx - Document creation"));
        assert!(desc.contains("browser - Web automation"));
    }

    #[test]
    fn test_skills_agent_has_coding_tools() {
        let agent = SkillsAgent::empty();
        let tools = agent.available_tools();

        // Should have same tools as CodingAgent
        assert!(tools.contains(&"list_files"));
        assert!(tools.contains(&"read_file"));
        assert!(tools.contains(&"edit_file"));
        assert!(tools.contains(&"delete_file"));
        assert!(tools.contains(&"write_file"));
        assert!(tools.contains(&"execute_shell"));
        assert!(tools.contains(&"grep"));
        assert!(tools.contains(&"todo_read"));
        assert!(tools.contains(&"todo_write"));
        assert!(tools.contains(&"todo_list"));
    }

    #[test]
    fn test_system_prompt_includes_venv_path() {
        let agent = SkillsAgent {
            skills: vec![],
            venv_path: PathBuf::from("/test/venvs/skills-venv"),
            skills_dir: PathBuf::from("/test/skills"),
            installed_tools: Vec::new(),
        };

        let prompt = agent.system_prompt();
        assert!(prompt.contains("/test/venvs/skills-venv"));
        assert!(prompt.contains("MUST use the designated interpreter"));
    }

    #[test]
    fn test_system_prompt_includes_skills() {
        let agent = SkillsAgent {
            skills: vec![SkillMetadata {
                name: "docx".to_string(),
                description: "Document creation".to_string(),
                license: "MIT".to_string(),
                path: PathBuf::from("/skills/docx"),
                skill_md_path: PathBuf::from("/skills/docx/SKILL.md"),
            }],
            venv_path: PathBuf::from("/venvs/skills-venv"),
            skills_dir: PathBuf::from("/skills"),
            installed_tools: Vec::new(),
        };

        let prompt = agent.system_prompt();
        assert!(prompt.contains("### docx"));
        assert!(prompt.contains("Document creation"));
        assert!(prompt.contains("READ THIS BEFORE USING"));
    }

    #[test]
    fn test_build_skills_section_empty() {
        let agent = SkillsAgent::empty();
        let section = agent.build_skills_section();

        assert!(section.contains("No skills are currently available"));
    }

    #[test]
    fn test_build_tools_section_empty() {
        let agent = SkillsAgent::empty();
        let section = agent.build_tools_section();

        assert!(section.contains("No external tools are currently installed"));
    }

    #[test]
    fn test_build_tools_section_with_tools() {
        let agent = SkillsAgent {
            skills: vec![],
            venv_path: PathBuf::from("/venvs/skills-venv"),
            skills_dir: PathBuf::from("/skills"),
            installed_tools: vec![
                InstalledTool {
                    name: "Pandoc".to_string(),
                    executable_path: PathBuf::from("/tools/pandoc/bin/pandoc"),
                },
                InstalledTool {
                    name: "Node.js".to_string(),
                    executable_path: PathBuf::from("/tools/node/bin/node"),
                },
            ],
        };

        let section = agent.build_tools_section();
        assert!(section.contains("**Pandoc**"));
        assert!(section.contains("/tools/pandoc/bin/pandoc"));
        assert!(section.contains("**Node.js**"));
        assert!(section.contains("/tools/node/bin/node"));
    }

    #[test]
    fn test_system_prompt_includes_tools() {
        let agent = SkillsAgent {
            skills: vec![],
            venv_path: PathBuf::from("/venvs/skills-venv"),
            skills_dir: PathBuf::from("/skills"),
            installed_tools: vec![InstalledTool {
                name: "LibreOffice".to_string(),
                executable_path: PathBuf::from("/tools/libreoffice/soffice.AppImage"),
            }],
        };

        let prompt = agent.system_prompt();
        assert!(prompt.contains("## External Tools"));
        assert!(prompt.contains("**LibreOffice**"));
        assert!(prompt.contains("/tools/libreoffice/soffice.AppImage"));
    }
}
