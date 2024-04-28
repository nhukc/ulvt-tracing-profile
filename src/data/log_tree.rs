// Copyright 2024 Ulvetanna Inc.

pub struct LogTree {
    pub label: String,
    pub children: Vec<LogTree>,
}

impl std::fmt::Display for LogTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.label)?;
        self.display_children(f, Vec::new())
    }
}

impl LogTree {
    fn display_children(&self, f: &mut std::fmt::Formatter, spaces: Vec<bool>) -> std::fmt::Result {
        for (i, child) in self.children.iter().enumerate() {
            let is_last = i == self.children.len() - 1;
            for is_space in &spaces {
                if *is_space {
                    write!(f, "   ")?;
                } else {
                    write!(f, "│  ")?;
                }
            }
            if is_last {
                writeln!(f, "└── {}", child.label)?;
            } else {
                writeln!(f, "├── {}", child.label)?;
            }
            if !child.children.is_empty() {
                let mut next_spaces = spaces.clone();
                next_spaces.push(is_last);
                child.display_children(f, next_spaces)?;
            }
        }
        Ok(())
    }
}
