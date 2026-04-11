//! Test Generator Service
//!
//! Uses AI to generate unit and integration tests for source code.

use crate::services::claude_cli::call_claude_cli;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Supported test frameworks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestFramework {
    Rust,
    PythonPytest,
    JavaScriptJest,
    TypeScriptJest,
    GoTest,
    JavaJUnit,
}

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
}

impl Language {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "rs" => Some(Language::Rust),
            "py" => Some(Language::Python),
            "js" => Some(Language::JavaScript),
            "ts" => Some(Language::TypeScript),
            "go" => Some(Language::Go),
            "java" => Some(Language::Java),
            _ => None,
        }
    }
}

impl TestFramework {
    pub fn default_for_language(lang: Language) -> Self {
        match lang {
            Language::Rust => TestFramework::Rust,
            Language::Python => TestFramework::PythonPytest,
            Language::JavaScript => TestFramework::JavaScriptJest,
            Language::TypeScript => TestFramework::TypeScriptJest,
            Language::Go => TestFramework::GoTest,
            Language::Java => TestFramework::JavaJUnit,
        }
    }
}

/// Test generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateTestsRequest {
    /// Source code to generate tests for
    pub source_code: String,
    /// Programming language
    pub language: Language,
    /// Test framework to use
    #[serde(default)]
    pub framework: Option<TestFramework>,
    /// Optional file path for context
    #[serde(default)]
    pub file_path: Option<String>,
    /// Generate integration tests instead of unit tests
    #[serde(default)]
    pub integration: bool,
    /// Additional instructions for test generation
    #[serde(default)]
    pub additional_instructions: Option<String>,
}

/// Test generation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateTestsResponse {
    /// Generated test code
    pub test_code: String,
    /// Language of the generated tests
    pub language: Language,
    /// Framework used
    pub framework: TestFramework,
    /// Number of test functions/classes generated
    pub test_count: usize,
    /// Any warnings or notes about the generated tests
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Test generator errors
#[derive(Debug, Error)]
pub enum TestGenError {
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("AI generation failed: {0}")]
    GenerationFailed(String),

    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),

    #[error("Unsupported framework: {0}")]
    UnsupportedFramework(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Test generator service
pub struct TestGenerator {
    ai_registry: std::sync::Arc<nexus_ai::AIProviderRegistry>,
    default_model: String,
}

impl TestGenerator {
    /// Create a new test generator
    pub fn new(
        ai_registry: std::sync::Arc<nexus_ai::AIProviderRegistry>,
    ) -> Self {
        Self {
            ai_registry,
            default_model: "gpt-4".to_string(),
        }
    }

    /// Create with a specific model
    pub fn with_model(mut self, model: String) -> Self {
        self.default_model = model;
        self
    }

    /// Generate tests for source code
    pub async fn generate_tests(
        &self,
        request: GenerateTestsRequest,
    ) -> Result<GenerateTestsResponse, TestGenError> {
        let framework = request
            .framework
            .unwrap_or_else(|| TestFramework::default_for_language(request.language));

        let prompt = self.build_prompt(&request, framework);

        let test_code = self.call_ai(&prompt).await?;

        let test_count = self.count_tests(&test_code, framework);

        Ok(GenerateTestsResponse {
            test_code,
            language: request.language,
            framework,
            test_count,
            warnings: Vec::new(),
        })
    }

    /// Generate unit tests
    pub async fn generate_unit_tests(
        &self,
        source_code: &str,
        language: Language,
    ) -> Result<GenerateTestsResponse, TestGenError> {
        self.generate_tests(GenerateTestsRequest {
            source_code: source_code.to_string(),
            language,
            framework: None,
            file_path: None,
            integration: false,
            additional_instructions: None,
        })
        .await
    }

    /// Generate integration tests
    pub async fn generate_integration_tests(
        &self,
        source_code: &str,
        language: Language,
    ) -> Result<GenerateTestsResponse, TestGenError> {
        self.generate_tests(GenerateTestsRequest {
            source_code: source_code.to_string(),
            language,
            framework: None,
            file_path: None,
            integration: true,
            additional_instructions: Some(
                "Focus on testing the interaction between components, "
                    .to_string()
                    + "database operations, API calls, and external service integrations.",
            ),
        })
        .await
    }

    /// Build the prompt for test generation
    fn build_prompt(
        &self,
        request: &GenerateTestsRequest,
        framework: TestFramework,
    ) -> String {
        let test_type = if request.integration {
            "integration tests"
        } else {
            "unit tests"
        };

        let framework_name = match framework {
            TestFramework::Rust => "Rust with #[test] and #[cfg(test)]",
            TestFramework::PythonPytest => "Python with pytest",
            TestFramework::JavaScriptJest => "JavaScript with Jest",
            TestFramework::TypeScriptJest => "TypeScript with Jest",
            TestFramework::GoTest => "Go with testing package",
            TestFramework::JavaJUnit => "Java with JUnit 5",
        };

        let mut prompt = format!(
            r#"Generate {} for the following {} code using {}.

Source code:
```{}
{}
```

Requirements:
"#,
            test_type,
            match request.language {
                Language::Rust => "Rust",
                Language::Python => "Python",
                Language::JavaScript => "JavaScript",
                Language::TypeScript => "TypeScript",
                Language::Go => "Go",
                Language::Java => "Java",
            },
            framework_name,
            match request.language {
                Language::Rust => "rust",
                Language::Python => "python",
                Language::JavaScript => "javascript",
                Language::TypeScript => "typescript",
                Language::Go => "go",
                Language::Java => "java",
            },
            request.source_code
        );

        if request.integration {
            prompt.push_str(
                r#"
- Test the component's interaction with other components
- Test database operations if applicable
- Test API calls and external service integrations
- Use mocks/stubs where appropriate for external dependencies
"#,
            );
        } else {
            prompt.push_str(
                r#"
- Cover all public functions/methods
- Test edge cases and error conditions
- Include both success and failure scenarios
- Follow testing best practices for the framework
"#,
            );
        }

        if let Some(ref instructions) = request.additional_instructions {
            prompt.push_str(&format!("\nAdditional instructions: {}\n", instructions));
        }

        prompt.push_str(&format!(
            r#"
Please provide only the test code, without explanations.
Generate complete, working test code that can be run immediately.
"#,
        ));

        prompt
    }

    /// Call the AI to generate tests (via Claude CLI)
    async fn call_ai(&self, prompt: &str) -> Result<String, TestGenError> {
        // Auto-yes prefix to skip confirmation prompts
        let auto_yes_prefix = "You are operating in auto-yes mode. If you ask any question requiring confirmation (yes/no, continue?, proceed?, etc.), always assume the answer is YES and proceed automatically. Never ask for confirmation.";

        // Wrap prompt with auto-yes prefix
        let full_prompt = format!("{}\n\n{}", auto_yes_prefix, prompt);

        // 通过 Claude CLI 执行（Claude Switch 切换后自动使用新模型）
        let content = call_claude_cli(&full_prompt)
            .await
            .map_err(|e| TestGenError::GenerationFailed(e))?;

        // If the response contains code blocks, extract them
        if content.contains("```") {
            return self.extract_code_blocks(&content);
        }

        Ok(content)
    }

    /// Extract code from markdown code blocks
    fn extract_code_blocks(&self, content: &str) -> Result<String, TestGenError> {
        let mut result = String::new();
        let mut in_code_block = false;
        let mut current_lang = String::new();

        for line in content.lines() {
            if line.starts_with("```") {
                if in_code_block {
                    // End of code block
                    result.push('\n');
                    in_code_block = false;
                } else {
                    // Start of code block, extract language
                    in_code_block = true;
                    current_lang = line.trim_start_matches("```").to_string();
                }
            } else if in_code_block {
                result.push_str(line);
                result.push('\n');
            }
        }

        if result.is_empty() {
            // No code blocks found, return original content
            return Ok(content.trim().to_string());
        }

        Ok(result.trim().to_string())
    }

    /// Count the number of tests in generated code
    fn count_tests(&self, code: &str, framework: TestFramework) -> usize {
        match framework {
            TestFramework::Rust => {
                // Count #[test] attributes
                code.matches("#[test]").count()
            }
            TestFramework::PythonPytest => {
                // Count def test_ or async def test_
                code.matches("def test_")
                    .count()
                    + code.matches("async def test_").count()
            }
            TestFramework::JavaScriptJest | TestFramework::TypeScriptJest => {
                // Count it(' or test(' or describe(
                code.matches("it('")
                    .count()
                    + code.matches("it(\"")
                    .count()
                    + code.matches("test('")
                    .count()
                    + code.matches("test(\"")
                    .count()
                    + code.matches("describe('").count()
                    + code.matches("describe(\"").count()
            }
            TestFramework::GoTest => {
                // Count func Test
                code.matches("func Test")
                    .count()
            }
            TestFramework::JavaJUnit => {
                // Count @Test annotations
                code.matches("@Test").count()
            }
        }
    }
}

/// Integration with execution service
impl std::fmt::Display for TestFramework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestFramework::Rust => write!(f, "rust"),
            TestFramework::PythonPytest => write!(f, "pytest"),
            TestFramework::JavaScriptJest => write!(f, "jest"),
            TestFramework::TypeScriptJest => write!(f, "jest"),
            TestFramework::GoTest => write!(f, "go test"),
            TestFramework::JavaJUnit => write!(f, "junit"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_extension() {
        assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
        assert_eq!(Language::from_extension("py"), Some(Language::Python));
        assert_eq!(Language::from_extension("js"), Some(Language::JavaScript));
        assert_eq!(Language::from_extension("ts"), Some(Language::TypeScript));
        assert_eq!(Language::from_extension("go"), Some(Language::Go));
        assert_eq!(Language::from_extension("java"), Some(Language::Java));
        assert_eq!(Language::from_extension("unknown"), None);
    }

    #[test]
    fn test_framework_default_for_language() {
        assert_eq!(
            TestFramework::default_for_language(Language::Rust),
            TestFramework::Rust
        );
        assert_eq!(
            TestFramework::default_for_language(Language::Python),
            TestFramework::PythonPytest
        );
        assert_eq!(
            TestFramework::default_for_language(Language::JavaScript),
            TestFramework::JavaScriptJest
        );
        assert_eq!(
            TestFramework::default_for_language(Language::TypeScript),
            TestFramework::TypeScriptJest
        );
    }

    #[test]
    fn test_count_rust_tests() {
        let generator = TestGenerator::new(std::sync::Arc::new(
            nexus_ai::AIProviderRegistry::new(),
        ));

        let code = r#"
#[test]
fn test_addition() {
    assert_eq!(2 + 2, 4);
}

#[test]
fn test_subtraction() {
    assert_eq!(5 - 3, 2);
}
"#;

        assert_eq!(generator.count_tests(code, TestFramework::Rust), 2);
    }

    #[test]
    fn test_count_python_tests() {
        let generator = TestGenerator::new(std::sync::Arc::new(
            nexus_ai::AIProviderRegistry::new(),
        ));

        let code = r#"
def test_addition():
    assert 2 + 2 == 4

async def test_async():
    result = await fetch_data()
    assert result is not None

def test_subtraction():
    assert 5 - 3 == 2
"#;

        assert_eq!(
            generator.count_tests(code, TestFramework::PythonPytest),
            3
        );
    }

    #[test]
    fn test_extract_code_blocks() {
        let generator = TestGenerator::new(std::sync::Arc::new(
            nexus_ai::AIProviderRegistry::new(),
        ));

        let content = r#"
Here is the test code:

```rust
#[test]
fn test_example() {
    assert!(true);
}
```

That's it!
"#;

        let result = generator.extract_code_blocks(content).unwrap();
        assert!(result.contains("#[test]"));
        assert!(result.contains("fn test_example()"));
    }

    #[test]
    fn test_extract_code_blocks_no_blocks() {
        let generator = TestGenerator::new(std::sync::Arc::new(
            nexus_ai::AIProviderRegistry::new(),
        ));

        let content = "Just plain text without code blocks.";

        let result = generator.extract_code_blocks(content).unwrap();
        assert_eq!(result, content);
    }
}
