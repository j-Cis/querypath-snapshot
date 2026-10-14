// ./src/lib.rs (querypath-snapshot v1.1.0)
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};
use querypath::{PathsPatterns, PattEnvIndex, QueryResults};
use querypath_fmt::{BundleResult, QueryPathFmt};
use temporal_fmt::Temporal;

pub const DEFAULT_MAX_SINGLE_FILE_SIZE: u64 = 512 * 1024; // 512 KiB

struct EmptyEnv;
impl PattEnvIndex for EmptyEnv {
	fn has_dir(&self, _dir: &str) -> bool {
		false
	}
	fn has_file_with_prefix(&self, _prefix: &str) -> bool {
		false
	}
	fn any_file_in_dir(&self, _dir: &str, _check: &mut dyn FnMut(&str) -> bool) -> bool {
		false
	}
}

/// Makro umożliwiające intuicyjną składnię mapowania rozszerzeń:
/// `lang_map! { "rs" => "rust", "toml" => "toml" }`
#[macro_export]
macro_rules! lang_map {
	($($ext:expr => $lang:expr ),*$(,)?) => {
		vec![$( ($ext,$lang) ),*]
	};
}

/// Główna struktura konfiguracyjna do generowania migawek (snapshots) projektu.
#[derive(Debug, Clone)]
pub struct Snapshot {
	title: String,
	title_suffix_date_fmt: Option<String>,
	name: String,
	name_suffix_date_fmt: Option<String>,
	output_dir: PathBuf,
	max_single_file_size: u64,
	custom_lang_map: HashMap<String, String>,
	content_skip_patterns: Vec<String>,
	add_footer_self_promotion: bool,
	add_footer_summary_setups: bool,
	header_customize_text: Option<String>,
	footer_customize_text: Option<String>,
}

impl Default for Snapshot {
	fn default() -> Self {
		Self {
			title: "CODE SNAPSHOT".to_string(),
			title_suffix_date_fmt: None,
			name: "snapshot_".to_string(),
			name_suffix_date_fmt: Some("YYYY-MM-MD_hhmm".to_string()),
			output_dir: PathBuf::from("./dump"),
			max_single_file_size: DEFAULT_MAX_SINGLE_FILE_SIZE,
			custom_lang_map: HashMap::new(),
			content_skip_patterns: Vec::new(),
			add_footer_self_promotion: false,
			add_footer_summary_setups: false,
			header_customize_text: None,
			footer_customize_text: None,
		}
	}
}

impl Snapshot {
	#[must_use]
	pub fn new() -> Self {
		Self::default()
	}

	#[must_use]
	pub fn title(mut self, title: impl Into<String>) -> Self {
		self.title = title.into();
		self
	}

	/// Szablon formatu daty doklejany na końcu tytułu raportu H1.
	#[must_use]
	pub fn title_sufix_date_fmt(mut self, fmt: impl Into<String>) -> Self {
		self.title_suffix_date_fmt = Some(fmt.into());
		self
	}

	#[must_use]
	pub fn title_suffix_date_fmt(self, fmt: impl Into<String>) -> Self {
		self.title_sufix_date_fmt(fmt)
	}

	/// Prefiks nazwy pliku raportu (np. `_4Gen_`).
	#[must_use]
	pub fn name(mut self, name: impl Into<String>) -> Self {
		self.name = name.into();
		self
	}

	/// Szablon formatu daty doklejany do nazwy pliku przed `.md`.
	#[must_use]
	pub fn name_sufix_date_fmt(mut self, fmt: impl Into<String>) -> Self {
		self.name_suffix_date_fmt = Some(fmt.into());
		self
	}

	#[must_use]
	pub fn name_suffix_date_fmt(self, fmt: impl Into<String>) -> Self {
		self.name_sufix_date_fmt(fmt)
	}

	#[must_use]
	pub fn output_dir(mut self, dir: impl AsRef<Path>) -> Self {
		self.output_dir = dir.as_ref().to_path_buf();
		self
	}

	#[must_use]
	pub fn max_single_file_size(mut self, bytes: u64) -> Self {
		self.max_single_file_size = bytes;
		self
	}

	/// Wzorce ścieżek, których treść kodu ma zostać pominięta w raporcie (np. `"./assets/**"`).
	#[must_use]
	pub fn content_skip<I, S>(mut self, patterns: I) -> Self
	where
		I: IntoIterator<Item = S>,
		S: Into<String>,
	{
		self.content_skip_patterns.extend(patterns.into_iter().map(Into::into));
		self
	}

	/// Rejestruje mapowania rozszerzeń na identyfikatory języków Markdown.
	/// Przyjmuje kolekcję par `("rs", "rust")` lub wynik makra `lang_map!`.
	#[must_use]
	pub fn mapping_lang<I, K, V>(mut self, mappings: I) -> Self
	where
		I: IntoIterator<Item = (K, V)>,
		K: Into<String>,
		V: Into<String>,
	{
		for (ext, lang) in mappings {
			self.custom_lang_map.insert(ext.into().to_lowercase(), lang.into());
		}
		self
	}

	/// Pojedyncze mapowanie rozszerzenia na język Markdown.
	#[must_use]
	pub fn map_lang(mut self, ext: impl Into<String>, lang: impl Into<String>) -> Self {
		self.custom_lang_map.insert(ext.into().to_lowercase(), lang.into());
		self
	}

	/// Przełącznik autoreklamy w stopce raportu.
	#[must_use]
	pub fn add_footer_self_promotion(mut self, enable: bool) -> Self {
		self.add_footer_self_promotion = enable;
		self
	}

	/// Przełącznik tabelarycznego podsumowania parametrów i konfiguracji skanowania w stopce.
	#[must_use]
	pub fn add_footer_summary_setups(mut self, enable: bool) -> Self {
		self.add_footer_summary_setups = enable;
		self
	}

	/// Dowolny akapit tekstu wstrzykiwany w stopce dokumentu.
	#[must_use]
	pub fn add_fotter_customize_text(mut self, text: impl Into<String>) -> Self {
		self.footer_customize_text = Some(text.into());
		self
	}

	#[must_use]
	pub fn add_footer_customize_text(self, text: impl Into<String>) -> Self {
		self.add_fotter_customize_text(text)
	}

	/// Dowolny akapit tekstu wstrzykiwany bezpośrednio po nagłówku H1.
	#[must_use]
	pub fn add_header_customize_text(mut self, text: impl Into<String>) -> Self {
		self.header_customize_text = Some(text.into());
		self
	}

	/// Buduje pełny dokument Markdown na podstawie wyników, bundle i formatowania.
	pub fn build_report(&self, res: &QueryResults, bundle: &BundleResult, fmt: &QueryPathFmt) -> String {
		let now = SystemTime::now();

		let title_date = self
			.title_suffix_date_fmt
			.as_ref()
			.map(|f| Temporal::format_system_time(now, f))
			.unwrap_or_default();
		let full_title = format!("{}{}", self.title, title_date);

		let tree_rendered = fmt.format(bundle);

		let mut doc = String::new();

		// 1. Tytuł H1
		doc.push_str(&format!("# {}\n\n", full_title));

		// 2. Opcjonalny własny akapit bezpośrednio po H1
		if let Some(ref header_custom) = self.header_customize_text {
			doc.push_str(header_custom.trim());
			doc.push_str("\n\n");
		}

		// 3. Struktura drzewa
		doc.push_str("## Structure\n\n```plaintext\n");
		doc.push_str(&tree_rendered);
		doc.push_str("\n```\n\n");

		// 4. Treść plików źródłowych
		doc.push_str("## Source Code Content\n\n");

		let skip_matcher = PathsPatterns::build(&self.content_skip_patterns, false).ok();
		let env = EmptyEnv;

		for file in bundle.text_files() {
			doc.push_str(&format!("### [{}] `{}`\n\n", file.number, file.path));

			let is_skipped = skip_matcher
				.as_ref()
				.map(|m| m.is_match(&file.path, &env))
				.unwrap_or(false);

			if is_skipped {
				doc.push_str("> *(File content skipped: belongs to content skip filter)*\n\n");
			} else if file.size > self.max_single_file_size {
				doc.push_str(&format!(
					"> *(File skipped: size {} B exceeds limit of {} B)*\n\n",
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
						let lang = self.resolve_lang(ext);
						doc.push_str(&format!("```{}\n{}\n```\n\n", lang, content));
					}
					Err(_) => {
						doc.push_str("> *(Error reading file content)*\n\n");
					}
				}
			}
		}

		// 5. Powtórzona struktura
		doc.push_str("## End Structure Summary\n\n```plaintext\n");
		doc.push_str(&tree_rendered);
		doc.push_str("\n```\n\n");

		// 6. Opcjonalne tabelaryczne podsumowanie konfiguracji
		if self.add_footer_summary_setups {
			doc.push_str("## Configuration & Setups Summary\n\n");
			doc.push_str("| Setup Parameter | Configuration Value |\n");
			doc.push_str("| :--- | :--- |\n");
			doc.push_str(&format!("| **Execution Dir (CWD)** | `{}` |\n", res.execution_dir));
			doc.push_str(&format!("| **Scan Targets** | `{:?}` |\n", res.scanned_paths));
			doc.push_str(&format!("| **Match Patterns** | `{:?}` |\n", res.patterns));
			doc.push_str(&format!("| **Content Skip Patterns** | `{:?}` |\n", self.content_skip_patterns));
			doc.push_str(&format!("| **Max Single File Size** | `{} B` |\n", self.max_single_file_size));
			doc.push_str(&format!("| **Scanned Objects** | `{} files, {} dirs` |\n", res.scanned_files, res.scanned_dirs));
			doc.push_str(&format!("| **Execution Time** | `{} ms` |\n\n", res.duration_ms));
		}

		// 7. Opcjonalny własny akapit w stopce
		if let Some(ref footer_custom) = self.footer_customize_text {
			doc.push_str(footer_custom.trim());
			doc.push_str("\n\n");
		}

		// 8. Opcjonalna autoreklama
		if self.add_footer_self_promotion {
			doc.push_str("---\n*Generated automatically by querypath-snapshot via j-Cis/{querypath,querypath-fmt,temporal-fmt} (crates.io | github.com)*\n");
		}

		doc
	}

	/// Generuje raport oraz zapisuje go na dysku.
	pub fn generate_and_save(
		&self,
		res: &QueryResults,
		bundle: &BundleResult,
		fmt: &QueryPathFmt,
	) -> Result<PathBuf> {
		let now = SystemTime::now();

		let name_date = self
			.name_suffix_date_fmt
			.as_ref()
			.map(|f| Temporal::format_system_time(now, f))
			.unwrap_or_default();
		let file_name = format!("{}{}.md", self.name, name_date);

		if !self.output_dir.exists() {
			fs::create_dir_all(&self.output_dir)
				.with_context(|| format!("Nie udało się utworzyć folderu: {:?}", self.output_dir))?;
		}

		let target_path = self.output_dir.join(file_name);
		let content = self.build_report(res, bundle, fmt);

		fs::write(&target_path, content)
			.with_context(|| format!("Błąd zapisu pliku do: {:?}", target_path))?;

		Ok(target_path)
	}

	fn resolve_lang(&self, ext: &str) -> &str {
		let clean = ext.to_lowercase();
		if let Some(custom) = self.custom_lang_map.get(&clean) {
			return custom.as_str();
		}
		match clean.as_str() {
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