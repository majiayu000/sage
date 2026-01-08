//! Nerd Font icons for beautiful CLI display
//!
//! Icons from Nerd Fonts: https://www.nerdfonts.com/cheat-sheet
//! Fallback ASCII versions provided for terminals without Nerd Fonts

/// Icon set with Nerd Font and ASCII fallbacks
pub struct Icons;

impl Icons {
    // === Application ===
    pub const SAGE: &'static str = "󰚩";           // nf-md-robot
    pub const SAGE_ASCII: &'static str = "[S]";

    // === Git ===
    pub const GIT_BRANCH: &'static str = "";     // nf-oct-git_branch
    pub const GIT_BRANCH_ASCII: &'static str = "⎇";

    // === Folder/Directory ===
    pub const FOLDER: &'static str = "";         // nf-oct-file_directory
    pub const FOLDER_ASCII: &'static str = "📁";

    // === Model/AI ===
    pub const MODEL: &'static str = "󰧑";          // nf-md-head_cog
    pub const MODEL_ASCII: &'static str = "🤖";

    // === Status ===
    pub const SUCCESS: &'static str = "";        // nf-fa-check
    pub const SUCCESS_ASCII: &'static str = "✓";

    pub const ERROR: &'static str = "";          // nf-fa-times
    pub const ERROR_ASCII: &'static str = "✗";

    pub const WARNING: &'static str = "";        // nf-fa-warning
    pub const WARNING_ASCII: &'static str = "⚠";

    pub const INFO: &'static str = "";           // nf-fa-info_circle
    pub const INFO_ASCII: &'static str = "ℹ";

    pub const THINKING: &'static str = "󰔟";       // nf-md-timer_sand
    pub const THINKING_ASCII: &'static str = "⏳";

    pub const RUNNING: &'static str = "";        // nf-fa-spinner (or use 󰑮)
    pub const RUNNING_ASCII: &'static str = "▶";

    // === Tools ===
    pub const TOOL: &'static str = "";           // nf-fa-wrench
    pub const TOOL_ASCII: &'static str = "🔧";

    pub const TERMINAL: &'static str = "";       // nf-oct-terminal
    pub const TERMINAL_ASCII: &'static str = "$";

    pub const CODE: &'static str = "";           // nf-fa-code
    pub const CODE_ASCII: &'static str = "<>";

    pub const FILE: &'static str = "";           // nf-oct-file
    pub const FILE_ASCII: &'static str = "📄";

    pub const SEARCH: &'static str = "";         // nf-fa-search
    pub const SEARCH_ASCII: &'static str = "🔍";

    pub const EDIT: &'static str = "";           // nf-fa-pencil
    pub const EDIT_ASCII: &'static str = "✎";

    // === Session/History ===
    pub const HISTORY: &'static str = "";        // nf-fa-history
    pub const HISTORY_ASCII: &'static str = "⏱";

    pub const SESSION: &'static str = "󰆍";        // nf-md-message_text
    pub const SESSION_ASCII: &'static str = "💬";

    // === Tree structure ===
    pub const TREE_BRANCH: &'static str = "├──";
    pub const TREE_LAST: &'static str = "└──";
    pub const TREE_VERTICAL: &'static str = "│";

    // === Prompt ===
    pub const PROMPT: &'static str = "❯";
    pub const PROMPT_ASCII: &'static str = ">";

    // === Tokens/Stats ===
    pub const TOKEN_IN: &'static str = "󰁍";       // nf-md-arrow_down_bold
    pub const TOKEN_IN_ASCII: &'static str = "↓";

    pub const TOKEN_OUT: &'static str = "󰁝";      // nf-md-arrow_up_bold
    pub const TOKEN_OUT_ASCII: &'static str = "↑";

    pub const CLOCK: &'static str = "";          // nf-fa-clock_o
    pub const CLOCK_ASCII: &'static str = "⏱";

    // === Misc ===
    pub const SPARKLE: &'static str = "";        // nf-oct-sparkle (or 󰛨)
    pub const SPARKLE_ASCII: &'static str = "✨";

    pub const LIGHTNING: &'static str = "";      // nf-oct-zap
    pub const LIGHTNING_ASCII: &'static str = "⚡";

    pub const HELP: &'static str = "";           // nf-fa-question_circle
    pub const HELP_ASCII: &'static str = "?";
}

/// Icon provider that can switch between Nerd Font and ASCII modes
#[derive(Debug, Clone, Copy)]
pub struct IconProvider {
    use_nerd_fonts: bool,
}

impl Default for IconProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl IconProvider {
    /// Create a new icon provider (defaults to Nerd Fonts enabled)
    pub const fn new() -> Self {
        Self {
            use_nerd_fonts: true,
        }
    }

    /// Create an ASCII-only icon provider
    pub const fn ascii() -> Self {
        Self {
            use_nerd_fonts: false,
        }
    }

    /// Check if using Nerd Fonts
    pub const fn is_nerd_fonts(&self) -> bool {
        self.use_nerd_fonts
    }

    // === Icon getters ===

    pub const fn sage(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::SAGE } else { Icons::SAGE_ASCII }
    }

    pub const fn git_branch(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::GIT_BRANCH } else { Icons::GIT_BRANCH_ASCII }
    }

    pub const fn folder(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::FOLDER } else { Icons::FOLDER_ASCII }
    }

    pub const fn model(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::MODEL } else { Icons::MODEL_ASCII }
    }

    pub const fn success(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::SUCCESS } else { Icons::SUCCESS_ASCII }
    }

    pub const fn error(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::ERROR } else { Icons::ERROR_ASCII }
    }

    pub const fn warning(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::WARNING } else { Icons::WARNING_ASCII }
    }

    pub const fn info(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::INFO } else { Icons::INFO_ASCII }
    }

    pub const fn thinking(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::THINKING } else { Icons::THINKING_ASCII }
    }

    pub const fn running(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::RUNNING } else { Icons::RUNNING_ASCII }
    }

    pub const fn tool(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::TOOL } else { Icons::TOOL_ASCII }
    }

    pub const fn terminal(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::TERMINAL } else { Icons::TERMINAL_ASCII }
    }

    pub const fn code(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::CODE } else { Icons::CODE_ASCII }
    }

    pub const fn file(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::FILE } else { Icons::FILE_ASCII }
    }

    pub const fn search(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::SEARCH } else { Icons::SEARCH_ASCII }
    }

    pub const fn edit(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::EDIT } else { Icons::EDIT_ASCII }
    }

    pub const fn history(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::HISTORY } else { Icons::HISTORY_ASCII }
    }

    pub const fn session(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::SESSION } else { Icons::SESSION_ASCII }
    }

    pub const fn prompt(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::PROMPT } else { Icons::PROMPT_ASCII }
    }

    pub const fn token_in(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::TOKEN_IN } else { Icons::TOKEN_IN_ASCII }
    }

    pub const fn token_out(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::TOKEN_OUT } else { Icons::TOKEN_OUT_ASCII }
    }

    pub const fn clock(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::CLOCK } else { Icons::CLOCK_ASCII }
    }

    pub const fn sparkle(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::SPARKLE } else { Icons::SPARKLE_ASCII }
    }

    pub const fn lightning(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::LIGHTNING } else { Icons::LIGHTNING_ASCII }
    }

    pub const fn help(&self) -> &'static str {
        if self.use_nerd_fonts { Icons::HELP } else { Icons::HELP_ASCII }
    }
}
