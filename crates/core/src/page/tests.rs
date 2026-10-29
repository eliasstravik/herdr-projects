//! The page's own tests: readability, escaping and the evidence guard.
use super::*;
use super::html::render;
use std::fs;

    #[test]
    fn markdown_is_readable_and_raw_html_never_runs() {
        let h = render("T & <x>", "report.md", Kind::Report, "# Head\n\nA **bold** [link](https://x/y) and `a<b>`.\n\n<script>alert(1)</script>\n\n## Tabla\n\n| A | B |\n|---|---|\n| 1 | 2 |\n", &[]);
        assert!(h.contains("<h1 id=\"head\">Head</h1>") && h.contains("<strong>bold</strong>") && h.contains("href=\"https://x/y\""));
        assert!(h.contains("&lt;script&gt;alert(1)&lt;/script&gt;") && !h.contains("<script>"));
        assert!(h.contains("<table>") && h.contains("<td>1</td>"));
        assert!(h.contains("Source Markdown (report.md)") && h.contains("No evidence files"));
        assert!(h.contains(MARKER));
    }

    #[test]
    fn an_agents_own_index_is_never_replaced_and_evidence_is_checked() {
        let dir = std::env::temp_dir().join(format!("sw-page-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir); fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("report.md"), "# Mine\n").unwrap();
        fs::write(dir.join("index.html"), "<!doctype html><h1>agent</h1>").unwrap();
        assert_eq!(ensure_page(&dir, "report.md", "T", Kind::Report), Some(dir.join("index.html")));
        assert_eq!(fs::read_to_string(dir.join("index.html")).unwrap(), "<!doctype html><h1>agent</h1>");
        fs::remove_file(dir.join("index.html")).unwrap();
        fs::write(dir.join("evidence.txt"), "ok\n").unwrap(); fs::write(dir.join("token.txt"), "ghp_abcdef\n").unwrap();
        assert_eq!(evidence_files(&dir), vec!["evidence.txt"]);
        assert!(ensure_page(&dir, "report.md", "T", Kind::Report).unwrap().is_file());
        assert!(fs::read_to_string(dir.join("index.html")).unwrap().contains("Evidence (evidence.txt)") || fs::read_to_string(dir.join("index.html")).unwrap().contains("evidence.txt"));
        let _ = fs::remove_dir_all(&dir);
    }
