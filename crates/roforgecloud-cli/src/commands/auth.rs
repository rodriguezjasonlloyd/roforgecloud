use std::cell::Cell;

use arboard::Clipboard;
use colored::Colorize;
use roforgecloud_core::auth::LoginPrompt;

#[derive(Default)]
pub struct CliLoginPrompt {
    lines: Cell<usize>,
}

impl CliLoginPrompt {
    fn clear_printed(&self) {
        for _ in 0..self.lines.get() {
            print!("\x1b[1A\x1b[2K");
        }
        self.lines.set(0);
    }

    fn println(&self, text: impl std::fmt::Display) {
        println!("{text}");
        self.lines.set(self.lines.get() + 1);
    }
}

impl LoginPrompt for CliLoginPrompt {
    fn auth_url(&self, url: &str) {
        self.println("Authorize roforgecloud".bold());

        let copied = Clipboard::new()
            .and_then(|mut c| c.set_text(url.to_string()))
            .is_ok();

        if copied {
            self.println(format!(
                "Open this link in your browser: {}",
                "(copied to clipboard)".dimmed()
            ));
        } else {
            self.println("Open this link in your browser:");
        }

        self.println(format!(
            "\x1b]8;;{url}\x1b\\{}\x1b]8;;\x1b\\",
            url.cyan().underline()
        ));
        self.println("");
    }

    fn browser_open_failed(&self) {
        self.println(
            "couldn't open a browser automatically, open the link above manually".yellow(),
        );
    }

    fn waiting(&self) {
        self.println("waiting for authorization...".dimmed());
    }

    fn success(&self) {
        self.clear_printed();
        println!("{}", "logged in".green());
    }
}
