use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use anyhow::{Context, Result};
use querypath::QueryResults;
use querypath_fmt::QueryPathFmt;
use temporal_fmt::Temporal;

pub const DEFAULT_MAX_SINGLE_FILE_SIZE: u64 = 512 * 1024; // 512 KiB

/// Główna struktura konfiguracyjna do generowania migawek (snapshots) projektu.
#[derive(Debug, Clone)]
pub struct Snapshot {
    title: String,
    output_dir: PathBuf,
    version_pattern: String,
    file_name_pattern: String,
    max_single_file_size: u64,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            title: "CODE SNAPSHOT".to_string(),
            output_dir: PathBuf::from("./prompt"),
            version_pattern: "YYYY-MM-MD_hhmm".to_string(),
            file_name_pattern: "snapshot_{VERSION}.md".to_string(),
            max_single_file_size: DEFAULT_MAX_SINGLE_FILE_SIZE,
        }
    }
}

impl Snapshot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn output_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.output_dir = dir.as_ref().to_path_buf();
        self
    }

    pub fn version_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.version_pattern = pattern.into();
        self
    }

    pub fn file_name_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.file_name_pattern = pattern.into();
        self
    }

    /// Ustawia maksymalny dopuszczalny rozmiar pojedynczego pliku źródłowego (w bajtach),
    /// którego treść zostanie wczytana i dołączona do raportu.
    /// Pliki przekraczające ten limit zostaną w raporcie opatrzone adnotacją o pominięciu zawartości.
    pub fn max_single_file_size(mut self, bytes: u64) -> Self {
        self.max_single_file_size = bytes;
        self
    }

    /// Generuje treść raportu Markdown na podstawie wyników skanowania i konfiguracji formatowania.
    pub fn build_report(&self, res: &QueryResults, fmt: &QueryPathFmt) -> String {
        let version = Temporal::format_system_time(SystemTime::now(), &self.version_pattern);
        let tree_rendered = fmt.format(res);

        let mut doc = String::new();

        // 1. Tytuł raportu
        doc.push_str(&format!("# {} v:{}\n\n", self.title, version));

        // 2. Sekcja z informacjami o parametrach skanowania
        doc.push_str("## Metadata\n\n");
        doc.push_str("```text\n");
        doc.push_str(&format!("├─ Katalog roboczy (CWD): {}\n", res.execution_dir));
        doc.push_str(&format!("├─ Lokalizacje (scan_at): {:?}\n", res.scanned_paths));
        doc.push_str(&format!("└─ Wzorce (match_pattern): {:?}\n", res.patterns));
        doc.push_str(&format!(
            "📦 Zeskanowano fizycznie: {} plików, {} katalogów\n",
            res.scanned_files, res.scanned_dirs
        ));
        doc.push_str("```\n\n");

        // 3. Nagłówek struktury plików (SOTC - Structure of Content)
        doc.push_str("## Structure\n\n");
        doc.push_str("```plaintext\n");
        doc.push_str(&tree_rendered);
        doc.push_str("```\n\n");

        // 4. Zawartość plików tekstowych z numeracją
        doc.push_str("## Source Code Content\n\n");

        let mut text_files: Vec<_> = res.files.iter().filter(|f| f.is_binary == false).collect();
        text_files.sort_by(|a, b| a.path.cmp(&b.path));

        let mut file_counter = 1;
        for file in text_files {
            doc.push_str(&format!("### [{}] `{}`\n\n", file_counter, file.path));

            if file.size > self.max_single_file_size {
                doc.push_str(&format!(
                    "> *(Plik pominięty: rozmiar {} B przekracza dopuszczalny limit {} B dla pojedynczego pliku)*\n\n",
                    file.size, self.max_single_file_size
                ));
            } else {
                let full_path = Path::new(&res.execution_dir).join(&file.path);
                match fs::read_to_string(&full_path) {
                    Ok(content) => {
                        let ext = full_path
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("");
                        let lang = Self::map_ext_to_lang(ext);
                        doc.push_str(&format!("```{}\n{}\n```\n\n", lang, content));
                    }
                    Err(_) => {
                        doc.push_str("> *(Błąd odczytu zawartości pliku)*\n\n");
                    }
                }
            }
            file_counter += 1;
        }

        // 5. Ponowna struktura na końcu raportu dla szybkiego odniesienia
        doc.push_str("## End Structure Summary\n\n");
        doc.push_str("```plaintext\n");
        doc.push_str(&tree_rendered);
        doc.push_str("```\n\n");

        doc.push_str("---\n*Generated automatically by querypath-snapshot*\n");

        doc
    }

    /// Generuje raport oraz zapisuje go we wskazanym katalogu wyjściowym.
    pub fn generate_and_save(&self, res: &QueryResults, fmt: &QueryPathFmt) -> Result<PathBuf> {
        let version = Temporal::format_system_time(SystemTime::now(), &self.version_pattern);
        let file_name = self.file_name_pattern.replace("{VERSION}", &version);

        if self.output_dir.exists() == false {
            fs::create_dir_all(&self.output_dir)
                .with_context(|| format!("Nie udało się utworzyć katalogu: {:?}", self.output_dir))?;
        }

        let target_path = self.output_dir.join(file_name);
        let content = self.build_report(res, fmt);

        fs::write(&target_path, content)
            .with_context(|| format!("Nie udało się zapisać migawki do: {:?}", target_path))?;

        Ok(target_path)
    }

    fn map_ext_to_lang(ext: &str) -> &'static str {
        match ext.to_lowercase().as_str() {
            "rs" => "rust",
            "toml" => "toml",
            "md" => "markdown",
            "json" => "json",
            "yaml" | "yml" => "yaml",
            "html" => "html",
            "css" => "css",
            "js" => "javascript",
            "ts" => "typescript",
            "py" => "python",
            "sh" | "bash" => "bash",
            _ => "plaintext",
        }
    }
}