//! Central rule catalog.
//!
//! Every diagnostic code has a single entry here. The catalog drives:
//! - SARIF `tool.driver.rules[]` output
//! - config-file rule-severity overrides
//! - per-rule documentation in the README
//!
//! When a rule's meaning or default level changes, bump
//! [`crate::report::RULE_CATALOG_VERSION`].

use crate::diag::Severity;

#[derive(Clone, Copy, Debug)]
pub struct Rule {
    pub id: &'static str,
    pub name: &'static str,
    pub short_description: &'static str,
    pub full_description: &'static str,
    pub default_level: Severity,
}

impl Rule {
    pub fn help_uri(&self) -> String {
        format!(
            "https://github.com/Systems-Modeling/SysML-v2-Release/blob/master/rules/{}.md",
            self.id
        )
    }
}

pub const CATALOG: &[Rule] = &[
    Rule {
        id: "SYSML001",
        name: "InvalidControlCharacter",
        short_description: "Invalid control character in source text.",
        full_description: "Source text contains an ASCII control character outside of permitted whitespace. SysML v2 textual models must be valid UTF-8 with control characters limited to tab, carriage return, and line feed.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML002",
        name: "UnterminatedStringLiteral",
        short_description: "Unterminated string literal.",
        full_description: "A string literal opened with ' or \" was not closed before end of file.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML003",
        name: "UnterminatedBlockComment",
        short_description: "Unterminated block comment.",
        full_description: "A /* ... */ block comment was not closed before end of file.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML010",
        name: "UnsupportedFileExtension",
        short_description: "Unsupported file extension.",
        full_description: "Files must use the .sysml or .kerml extension to be validated.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML012",
        name: "FileNotUtf8",
        short_description: "Unable to read file as UTF-8 text.",
        full_description: "The file could not be read as UTF-8. SysML v2 textual models are UTF-8 encoded per the OMG specification.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML020",
        name: "UnmatchedClosingDelimiter",
        short_description: "Unmatched closing delimiter.",
        full_description: "A closing brace, parenthesis, or bracket has no matching opener.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML021",
        name: "UnclosedDelimiter",
        short_description: "Unclosed delimiter.",
        full_description: "An opening brace, parenthesis, or bracket was never closed before end of file.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML030",
        name: "ExpectedPackageAfterLibrary",
        short_description: "Expected 'package' after 'library'.",
        full_description: "The 'library' keyword must be followed by 'package' to introduce a library package.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML031",
        name: "AliasMissingFor",
        short_description: "Alias declaration must include 'for'.",
        full_description: "An 'alias' declaration must include the 'for' keyword identifying the aliased target.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML032",
        name: "DependencyMissingTo",
        short_description: "Dependency declaration must include a supplier after 'to'.",
        full_description: "A 'dependency' declaration must include the 'to' keyword identifying the supplier.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML033",
        name: "UsageMissingNameOrSpecialization",
        short_description: "Usage missing a declared name or specialization.",
        full_description: "A usage statement must declare a name or include a specialization (:, :>, :>>, etc.).",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML034",
        name: "DefinitionMissingName",
        short_description: "Definition missing a name.",
        full_description: "A definition statement (e.g. 'part def', 'attribute def') must declare an identifier name.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML035",
        name: "MissingStatementTerminator",
        short_description: "Missing ';' or '{' terminator.",
        full_description: "A statement must end with ';' or open a body with '{' to be syntactically valid.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML040",
        name: "UnresolvedReferenceInFile",
        short_description: "Identifier reference not declared in this file.",
        full_description: "With --strict, a referenced identifier is not declared in the current file. It may still resolve from imports or libraries; cross-file resolution lands in Phase 2.",
        default_level: Severity::Warning,
    },
    Rule {
        id: "SYSML041",
        name: "DuplicateMemberInScope",
        short_description: "Duplicate member name in lexical scope.",
        full_description: "Two members in the same lexical scope share a name. SysML v2 scope members must be uniquely named within their enclosing namespace.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML210",
        name: "SpecializationTargetMissing",
        short_description: "Specialization target does not resolve.",
        full_description: "A `:>` or `specializes` relationship names a target that is not declared in this file, not imported, not present in any other project file, and not present in the embedded standard library. Specialization requires its target to exist.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML211",
        name: "RedefinitionTargetMissing",
        short_description: "Redefinition target does not resolve.",
        full_description: "A `:>>` or `redefines` relationship names a target that is not declared in this file, not imported, not present in any other project file, and not present in the embedded standard library. Redefinition requires its target to exist.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML212",
        name: "SelfSpecialization",
        short_description: "Feature specializes itself.",
        full_description: "A feature uses `:>` or `specializes` to refer to its own declared name. Specialization cannot be reflexive; the inheritance graph would be degenerate.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML213",
        name: "SelfRedefinition",
        short_description: "Feature redefines itself.",
        full_description: "A feature uses `:>>` or `redefines` to refer to its own declared name. Redefinition cannot be reflexive.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML220",
        name: "SpecializationCycle",
        short_description: "Specialization graph contains a cycle.",
        full_description: "Two or more features form a cycle through `:>` / `specializes` relationships (possibly across multiple files). The Pilot Implementation rejects such graphs because they break classification reasoning. The cycle path is included in the message.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML050",
        name: "UnusedSuppression",
        short_description: "Suppression directive did not match any diagnostic.",
        full_description: "A '// sysml-validate: disable=...' directive was placed but did not match any diagnostic in scope. The directive can be removed.",
        default_level: Severity::Warning,
    },
    Rule {
        id: "SYSML060",
        name: "InvalidSuppressionDirective",
        short_description: "Suppression directive has invalid syntax.",
        full_description: "A '// sysml-validate:' comment contained an unrecognized directive form. See the README for accepted forms.",
        default_level: Severity::Warning,
    },
    Rule {
        id: "SYSML800",
        name: "InvalidConfig",
        short_description: "Configuration file is invalid.",
        full_description: "sysml-validate.toml could not be parsed or contained an unrecognized field.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML900",
        name: "OfficialCommandError",
        short_description: "Official command parse or setup error.",
        full_description: "The --official-command template could not be tokenized or did not include an executable.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML901",
        name: "OfficialBackendSpawnError",
        short_description: "Official validator could not be executed.",
        full_description: "The official validator child process could not be spawned. Verify the executable path and permissions.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML902",
        name: "OfficialBackendNonZeroExit",
        short_description: "Official validator returned a non-zero exit status.",
        full_description: "The official validator reported a failure. The child stdout/stderr is included in the message.",
        default_level: Severity::Error,
    },
    Rule {
        id: "SYSML903",
        name: "OfficialBackendInfo",
        short_description: "Official validator informational output.",
        full_description: "The official validator returned non-empty output on a successful run.",
        default_level: Severity::Info,
    },
    Rule {
        id: "SYSML904",
        name: "OfficialBackendTimeout",
        short_description: "Official validator exceeded --timeout and was terminated.",
        full_description: "The official validator did not exit within --timeout seconds and was killed. Increase --timeout or investigate the upstream tool.",
        default_level: Severity::Error,
    },
];

#[allow(dead_code)] // Part of the public catalog API; consumed by Phase 2.
pub fn lookup(code: &str) -> Option<&'static Rule> {
    CATALOG.iter().find(|rule| rule.id == code)
}
