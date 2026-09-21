// ./examples/demo.rs
use anyhow::Result;
use querypath::QueryPath;
use querypath_fmt::{
	CasePrecedence, CharClass, Column, DirWeightDisplay, GroupStrategy, NoExtPriority, NodeGroup, Numeration,
	QueryPathFmt, SameNamePriority, Sorting, StatsTemporal, StatsWeight, UnitSystem, WeightPrecision,
};
// use temporal_fmt;
use querypath_snapshot::Snapshot;

fn main() -> Result<()> {
	let res: querypath::QueryResults =
		QueryPath::new()
			.scan_at(["./"])
			.match_pattern([
				"./{examples|tests|src}/**",
				"./{Cargo.toml|README.md}"
			])
			.keep_parent(true)
			.run()?;

	println!("🔍 [querypath] Inicjalizacja skanowania...");
	println!(" ├─ Katalog roboczy (CWD): {}", res.execution_dir);
	println!(" ├─ Lokalizacje (scan_at): {:?}", res.scanned_paths);
	println!(" └─ Wzorce (match_pattern): {:?}", res.patterns);
	println!(
        "📦 Zeskanowano fizycznie: {} plików, {} katalogów\n",
        res.scanned_files, res.scanned_dirs
    );

	/*
	 * println!("\n✨ Dopasowane katalogi ({}):", res.dirs.len());
	 * for d in &res.dirs {
	 * 	println!(
	 * 		" 📁 {} (dopasowane: {} B, pełne: {} B, mod: {:?})",
	 * 		d.path, d.matched_size, d.real_size, d.modified_at
	 * 	);
	 * }
	 * 
	 * println!("\n✨ Dopasowane pliki ({}):", res.files.len());
	 * for f in &res.files {
	 * 	println!(" 📄 {} (rozmiar: {} B, binarny: {}, mod: {:?})", f.path, f.size, f.is_binary, f.modified_at);
	 * }
	 */

	let fmt = QueryPathFmt::new()
		.name_width(25)
		.path_width(35)
		.column_order_left([])
		.column_order_right([Column::Weight, Column::Temporal, Column::Path])
		.numeration(Numeration::new().enabled(true).numerate_dirs(false).numerate_binaries(false).start_from(1))
		.stats_weight(
			StatsWeight::new()
				.enabled(true)
				.unit_system(UnitSystem::Binary)
				.dir_display(DirWeightDisplay::MatchedOnly)
				.precision(WeightPrecision::Tenths),
		)
		.stats_temporal(StatsTemporal::new().enabled(true).pattern("WYYY-WW-D hh:mm:ss"))
		.sorting(
			Sorting::new()
				.enabled(true)
				.group_strategy(GroupStrategy::Custom(vec![
					NodeGroup::TextFile,
					NodeGroup::Directory,
					NodeGroup::BinaryFile,
				]))
				.same_name_priority(SameNamePriority::FileFirst)
				.no_ext_priority(NoExtPriority::Above)
				.ignore_leading_dot(true)
				.char_class_order([CharClass::Special, CharClass::Digit, CharClass::Letter])
				.case_precedence(CasePrecedence::Insensitive),
		);

	let output = fmt.format(&res);
	println!("{}", output);

	/*
	CO DALEJ ?

	MAMY NUMERACJĘ PRZY PLIKACH, W TEJ SAMEJ KOLEJNOŚCI CO NUMERY PRZY PLIKACH, JEŚLI PLIKI SĄ TEKSTOWE, SĄ DOŁACZANE DO RAPORTU

	JAKIE MAMY OPCJE

	WSKAZUJEMY ŚCIEŻKĘ NA PLIKI WYNIKOWE, DOMYŚLNIE ./prompt/snapshot_[wersja].md

	WSAKZUJEMY FORMAT WERSJI - używamy `temporal_fmt`

	domyślny template raportów	
	- tytuł raportu `# CODE SNAPSHOT v:[wersja]`
	- puste miejsce na wstawienie sekcji 
	- informacja o parametrach skanowania czyli
	```
	println!(" ├─ Katalog roboczy (CWD): {}", res.execution_dir);
	println!(" ├─ Lokalizacje (scan_at): {:?}", res.scanned_paths);
	println!(" └─ Wzorce (match_pattern): {:?}", res.patterns);
	println!("📦 Zeskanowano fizycznie: {} plików, {} katalogów", res.scanned_files, res.scanned_dirs);
	``` 
	- struktura plików w oknie `plaintext` bezpośredni rezultat kodu `fmt.format(&res)` lub `println!("{}", output);`
	- raport z plików kodu numer zgodnie z numerem pokazanym przy plikach w strukturze drzewa. (kążde zamknięte w okna codeblock z nagłówkiem `### [numer] [ścieżka pliku]` i zawartością pliku  (oczywiście tylko pliki tekstowe bez binarnych i bez folderów, a także nie większe pliki tekstowe niż LIMIT_WEIGHT_MAX, a jeśli jest zaduży to jest adnotacja że był))
	- ponownie struktura plików w oknie `plaintext` bezpośredni rezultat kodu `fmt.format(&res)` lub `println!("{}", output);`
	- koniec raportu. 	

	 */

	// 3. Generowanie i zapis migawki (Snapshot)
    let snapshot = Snapshot::new()
        .title("CODE SNAPSHOT")
        .output_dir("./prompt")
        .version_pattern("WYYYWWDSSShhmmssttqq")
        .file_name_pattern("snapshot_{VERSION}.md")
        .max_single_file_size(256 * 1024); // Maksymalny rozmiar POJEDYNCZEGO pliku źródłowego (256 KiB)

    let saved_path = snapshot.generate_and_save(&res, &fmt)?;

    println!("\n✅ Generowanie migawki zakończone sukcesem!");
    println!("📄 Zapisano do: {}", saved_path.display());

    Ok(())
}