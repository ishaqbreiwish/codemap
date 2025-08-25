# 🚀 CodeMap - Intelligent Codebase Onboarding & Analysis Tool

> **A professional-grade tool that helps developers quickly understand and onboard to any codebase. Provides architecture analysis, tech stack identification, complexity metrics, and AI-powered insights.**

[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/version-0.2.0-blue?style=for-the-badge)](https://github.com/ishaqbreiwish/codemap)

## ✨ Features

### 🏗️ **Architecture Analysis**

- **Pattern Detection**: Automatically identifies common architectural patterns (MVC, Clean Architecture, etc.)
- **Component Mapping**: Maps key components and their relationships
- **Data Flow Analysis**: Visualizes how data flows through the system
- **Confidence Scoring**: Provides confidence levels for architectural assessments

### 🛠️ **Tech Stack Identification**

- **Language Detection**: Identifies all programming languages used
- **Framework Recognition**: Detects frameworks, libraries, and tools
- **Database Analysis**: Identifies database technologies and patterns
- **Deployment Detection**: Recognizes containerization and deployment tools

### 📊 **Advanced Metrics**

- **Complexity Analysis**: Calculates cyclomatic and cognitive complexity
- **Quality Metrics**: Assesses code coverage, documentation, and maintainability
- **Technical Debt**: Identifies areas needing refactoring
- **Hotspot Detection**: Finds complex or problematic code areas

### 🎯 **Smart Entry Points**

- **Intelligent Ranking**: Uses AI and heuristics to rank file importance
- **Onboarding Path**: Provides optimal reading order for new developers
- **Context-Aware**: Considers project structure and patterns
- **Interactive Tours**: Guided exploration of codebase (coming soon)

### 🤖 **AI-Powered Insights**

- **Project Brief**: Generates comprehensive project overviews
- **Code Explanations**: Provides context for complex code sections
- **Best Practices**: Suggests improvements and patterns
- **Learning Paths**: Creates personalized onboarding guides

## 🚀 Quick Start

### Installation

```bash
# From source
git clone https://github.com/ishaqbreiwish/codemap.git
cd codemap
cargo install --path .

# Or with cargo
cargo install codemap-cli
```

### Basic Usage

```bash
# Initialize analysis for current project
codemap init

# Generate comprehensive analysis
codemap analyze

# Show project summary
codemap summary

# Export analysis report
codemap export --format json --output report.json
```

## 📋 Commands

| Command   | Description                              | Example                                    |
| --------- | ---------------------------------------- | ------------------------------------------ |
| `init`    | Initialize analysis for current codebase | `codemap init --name my-project`           |
| `analyze` | Generate comprehensive analysis          | `codemap analyze --detailed --format text` |
| `summary` | Display project overview                 | `codemap summary`                          |
| `tour`    | Interactive codebase exploration         | `codemap tour`                             |
| `config`  | Configure API keys and settings          | `codemap config --api-key sk-...`          |
| `diff`    | Compare with previous analysis           | `codemap diff`                             |
| `export`  | Export analysis to various formats       | `codemap export --format html`             |

## 🎨 Sample Output

```
================================================================================
🚀 CODEBASE ANALYSIS SUMMARY
================================================================================

📋 PROJECT INFORMATION
   Name: codemap-cli
   Size: 45 files, 2,847 lines
   Files: 45 | Lines: 2847 | Functions: 156

🏗️  ARCHITECTURE
   Pattern: Rust Cargo Project + CLI Application (confidence: 85.0%)
   Data Flow: Command → Parser → Handler → Analysis → Output

🛠️  TECH STACK
   Languages: Rust
   Frameworks: Cargo, Clap, Serde
   Tools: Git, Cargo
   Deployment: Cargo

🎯 KEY ENTRY POINTS
   1. src/main.rs - Primary application entry point
   2. src/analysis.rs - Core analysis engine
   3. src/models.rs - Data structures and models
   4. src/display.rs - Output formatting and display
   5. Cargo.toml - Project configuration and dependencies

📊 QUALITY METRICS
   Maintainability: 85.0%
   Technical Debt: 15.0%
   Test Coverage: 75.0%
   Documentation: 40.0%
================================================================================
```

## 🔧 Configuration

CodeMap uses a TOML configuration file (`.codemap/config.toml`):

```toml
[general]
default_analysis_files = 20
max_file_size = 100000
enable_ai_insights = true

[ai]
provider = "openai"
model = "gpt-4o-mini"
api_key = ""
max_tokens = 4000

[output]
colored_output = true
show_progress = true
detailed_mode = false

[analysis]
detect_architecture = true
identify_tech_stack = true
complexity_analysis = true
quality_metrics = true
```

## 🤖 AI Integration

CodeMap can leverage AI for enhanced analysis:

```bash
# Set your OpenAI API key
codemap config --api-key sk-your-key-here

# Run analysis with AI insights
codemap analyze --detailed
```

**AI Features:**

- Project brief generation
- Code explanation and context
- Architecture pattern recognition
- Best practice recommendations
- Personalized onboarding guides

## 📊 Supported Languages & Frameworks

### Languages

- **Rust** (Cargo, Actix, Rocket, Tokio)
- **Python** (Django, Flask, FastAPI, Poetry)
- **JavaScript/TypeScript** (Node.js, React, Vue, Express)
- **Go** (Gin, Echo, Fiber)
- **Java** (Spring Boot, Maven, Gradle)
- **C#** (.NET, ASP.NET Core)
- **PHP** (Laravel, Symfony, Composer)

### Frameworks & Tools

- **Web Frameworks**: React, Vue, Angular, Django, Flask, Express
- **Databases**: PostgreSQL, MySQL, MongoDB, Redis, SQLite
- **Testing**: Jest, PyTest, Cargo Test, JUnit
- **Build Tools**: Webpack, Vite, Cargo, npm, yarn
- **Deployment**: Docker, Kubernetes, Heroku, AWS

## 🏆 Use Cases

### For Teams

- **Onboarding**: Get new developers up to speed quickly
- **Code Reviews**: Understand changes in context
- **Architecture Reviews**: Assess system design and complexity
- **Documentation**: Generate comprehensive project documentation

### For Individuals

- **Learning**: Understand unfamiliar codebases
- **Contributing**: Find the right files to modify
- **Debugging**: Identify problematic areas quickly
- **Refactoring**: Plan improvements with confidence

### For Managers

- **Project Assessment**: Evaluate code quality and complexity
- **Team Planning**: Understand skill requirements
- **Risk Assessment**: Identify technical debt and hotspots
- **Progress Tracking**: Monitor codebase evolution

## 🔮 Roadmap

### v0.3.0 (Coming Soon)

- [ ] Interactive guided tours
- [ ] HTML/Markdown report generation
- [ ] Advanced complexity analysis
- [ ] Git history integration
- [ ] Team collaboration features

### v0.4.0 (Planned)

- [ ] Visual architecture diagrams
- [ ] Performance analysis
- [ ] Security vulnerability detection
- [ ] Integration with CI/CD pipelines
- [ ] Plugin system for custom analyzers

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup

```bash
git clone https://github.com/ishaqbreiwish/codemap.git
cd codemap
cargo build
cargo test
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [Rust](https://www.rust-lang.org/) for performance and reliability
- Uses [Clap](https://github.com/clap-rs/clap) for beautiful CLI interfaces
- Leverages [OpenAI](https://openai.com/) for intelligent code analysis
- Inspired by tools like [Sourcetrail](https://www.sourcetrail.com/) and [CodeSee](https://www.codesee.io/)

---

**Made with ❤️ for developers who want to understand codebases faster and better.**

_Star this repository if you find it helpful!_
