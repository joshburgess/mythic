//! Utilities for generating synthetic test sites for benchmarks.

use std::path::Path;

const WORDS: &[&str] = &[
    "the", "be", "to", "of", "and", "a", "in", "that", "have", "I",
    "it", "for", "not", "on", "with", "he", "as", "you", "do", "at",
    "this", "but", "his", "by", "from", "they", "we", "say", "her", "she",
    "or", "an", "will", "my", "one", "all", "would", "there", "their", "what",
    "so", "up", "out", "if", "about", "who", "get", "which", "go", "me",
    "when", "make", "can", "like", "time", "no", "just", "him", "know", "take",
    "people", "into", "year", "your", "good", "some", "could", "them", "see", "other",
    "than", "then", "now", "look", "only", "come", "its", "over", "think", "also",
    "back", "after", "use", "two", "how", "our", "work", "first", "well", "way",
    "even", "new", "want", "because", "any", "these", "give", "day", "most", "us",
];

const TAGS: &[&str] = &[
    "rust", "web", "programming", "tutorial", "guide", "performance",
    "design", "architecture", "testing", "devops",
];

/// Generate a synthetic test site with the given number of pages.
pub fn generate_site(dir: &Path, num_pages: usize, seed: u64) {
    let content_dir = dir.join("content");
    std::fs::create_dir_all(&content_dir).unwrap();

    let templates_dir = dir.join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();

    // Write a simple template
    std::fs::write(
        templates_dir.join("default.html"),
        "<!DOCTYPE html><html><head><title>{{ page.title }}</title></head><body>{{ content | safe }}</body></html>",
    ).unwrap();

    // Write config
    std::fs::write(
        dir.join("mythic.toml"),
        "title = \"Benchmark Site\"\nbase_url = \"http://localhost:3000\"\n",
    ).unwrap();

    // Generate pages
    let mut rng = SimpleRng::new(seed);

    for i in 0..num_pages {
        let category = match i % 5 {
            0 => "blog",
            1 => "docs",
            2 => "tutorials",
            3 => "news",
            _ => "misc",
        };

        let subdir = content_dir.join(category);
        std::fs::create_dir_all(&subdir).unwrap();

        let title = format!("Post {} about {}", i, WORDS[rng.next_usize() % WORDS.len()]);
        let date = format!("2024-{:02}-{:02}", (i % 12) + 1, (i % 28) + 1);
        let tag1 = TAGS[rng.next_usize() % TAGS.len()];
        let tag2 = TAGS[rng.next_usize() % TAGS.len()];

        let body = generate_markdown_body(&mut rng, 500);

        let content = format!(
            "---\ntitle: \"{title}\"\ndate: \"{date}\"\ntags:\n  - {tag1}\n  - {tag2}\n---\n{body}"
        );

        std::fs::write(subdir.join(format!("post-{i}.md")), content).unwrap();
    }
}

fn generate_markdown_body(rng: &mut SimpleRng, target_words: usize) -> String {
    let mut body = String::new();
    let mut word_count = 0;

    // Heading
    body.push_str(&format!("## {}\n\n", random_sentence(rng, 5)));
    word_count += 5;

    while word_count < target_words {
        let section_type = rng.next_usize() % 6;

        match section_type {
            0 => {
                // Paragraph
                let words = 30 + rng.next_usize() % 50;
                body.push_str(&random_sentence(rng, words));
                body.push_str("\n\n");
                word_count += words;
            }
            1 => {
                // Heading
                let level = 2 + rng.next_usize() % 3;
                body.push_str(&format!(
                    "{} {}\n\n",
                    "#".repeat(level),
                    random_sentence(rng, 4)
                ));
                word_count += 4;
            }
            2 => {
                // List
                let items = 3 + rng.next_usize() % 5;
                for _ in 0..items {
                    body.push_str(&format!("- {}\n", random_sentence(rng, 8)));
                    word_count += 8;
                }
                body.push('\n');
            }
            3 => {
                // Code block
                body.push_str("```rust\nfn example() {\n    let x = 42;\n    println!(\"{}\", x);\n}\n```\n\n");
                word_count += 10;
            }
            4 => {
                // Link
                body.push_str(&format!(
                    "See [{}](https://example.com/{}) for more details.\n\n",
                    random_sentence(rng, 3),
                    WORDS[rng.next_usize() % WORDS.len()]
                ));
                word_count += 8;
            }
            _ => {
                // Bold/italic paragraph
                let words = 20 + rng.next_usize() % 30;
                body.push_str(&format!(
                    "This is **important**: {}. And *note* that {}.\n\n",
                    random_sentence(rng, words / 2),
                    random_sentence(rng, words / 2)
                ));
                word_count += words + 4;
            }
        }
    }

    body
}

fn random_sentence(rng: &mut SimpleRng, word_count: usize) -> String {
    (0..word_count)
        .map(|_| WORDS[rng.next_usize() % WORDS.len()])
        .collect::<Vec<_>>()
        .join(" ")
}

/// Simple deterministic PRNG for reproducible benchmarks.
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        SimpleRng { state: seed }
    }

    fn next_usize(&mut self) -> usize {
        // xorshift64
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state as usize
    }
}
