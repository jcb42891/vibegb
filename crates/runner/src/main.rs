use clap::{Parser, ValueEnum};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use vibegb_core::{GameBoy, Rom, RomHeader};

const DEFAULT_MAX_STEPS: usize = 2_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum RunnerMode {
    Header,
    Exec,
}

#[derive(Debug, Parser)]
#[command(
    name = "vibegb-runner",
    about = "Headless ROM loader and validation runner for VibeGB"
)]
struct Cli {
    #[arg(short, long, value_name = "PATH", required_unless_present = "suite")]
    rom: Option<PathBuf>,

    #[arg(long, value_name = "PATH")]
    suite: Option<PathBuf>,

    #[arg(long, value_name = "PATH", requires = "suite")]
    rom_root: Option<PathBuf>,

    #[arg(long, value_enum, default_value_t = RunnerMode::Header)]
    mode: RunnerMode,

    #[arg(long, default_value_t = DEFAULT_MAX_STEPS)]
    max_steps: usize,

    #[arg(long, value_name = "TEXT")]
    expect_serial: Option<String>,

    #[arg(long)]
    expect_mooneye_pass: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CaseExpectation {
    SerialContains(String),
    MooneyePass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SuiteCase {
    label: String,
    rom_path: PathBuf,
    max_steps: usize,
    expectation: Option<CaseExpectation>,
}

fn main() {
    let cli = Cli::parse();
    match execute(cli) {
        Ok(output) => {
            println!("{output}");
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

fn execute(cli: Cli) -> Result<String, String> {
    let Cli {
        rom,
        suite,
        rom_root,
        mode,
        max_steps,
        expect_serial,
        expect_mooneye_pass,
    } = cli;

    if let Some(suite_path) = suite {
        if expect_serial.is_some() || expect_mooneye_pass {
            return Err(
                "--expect-serial/--expect-mooneye-pass cannot be used with --suite".to_string(),
            );
        }
        return execute_suite(&suite_path, rom_root.as_deref(), max_steps);
    }

    let rom_path = rom.ok_or_else(|| "missing required --rom argument".to_string())?;
    let rom_data = Rom::from_file(&rom_path).map_err(|err| format!("ROM load failed: {err}"))?;

    match mode {
        RunnerMode::Header => {
            if expect_serial.is_some() || expect_mooneye_pass {
                return Err("--expect-serial/--expect-mooneye-pass require --mode exec".to_string());
            }
            Ok(render_header(&rom_path, &rom_data.header))
        }
        RunnerMode::Exec => {
            let report = run_for_steps(&rom_data.data, max_steps)?;
            assert_expectations(
                &report,
                expect_serial.as_deref(),
                expect_mooneye_pass,
                "single ROM run",
            )?;
            Ok(render_exec_report(&rom_path, &rom_data.header, &report))
        }
    }
}

fn execute_suite(
    suite_path: &Path,
    rom_root: Option<&Path>,
    default_max_steps: usize,
) -> Result<String, String> {
    let suite_text = fs::read_to_string(suite_path).map_err(|err| {
        format!(
            "failed to read suite file '{}': {err}",
            suite_path.display()
        )
    })?;
    let cases = parse_suite(&suite_text, default_max_steps)?;
    let mut total = 0usize;
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut report = format!("Suite: {}", suite_path.display());

    for case in cases {
        total += 1;
        let rom_path = resolve_case_rom_path(&case.rom_path, suite_path, rom_root);
        match run_suite_case(&case, &rom_path) {
            Ok(run_report) => {
                passed += 1;
                let _ = writeln!(
                    report,
                    "\nPASS | {} | steps={} | serial={}",
                    case.label,
                    case.max_steps,
                    summarize_serial(&run_report.serial_output)
                );
            }
            Err(reason) => {
                failed += 1;
                let _ = writeln!(report, "\nFAIL | {} | {}", case.label, reason);
            }
        }
    }

    let _ = writeln!(
        report,
        "\nSummary: total={total} passed={passed} failed={failed}"
    );

    if failed == 0 {
        Ok(report)
    } else {
        Err(report)
    }
}

fn run_suite_case(case: &SuiteCase, rom_path: &Path) -> Result<ExecutionReport, String> {
    let rom = Rom::from_file(rom_path).map_err(|err| {
        format!(
            "{}: ROM load failed for '{}': {err}",
            case.label,
            rom_path.display()
        )
    })?;
    let report = run_for_steps(&rom.data, case.max_steps).map_err(|err| {
        format!(
            "{}: execution failed for '{}': {err}",
            case.label,
            rom_path.display()
        )
    })?;

    if let Some(expectation) = &case.expectation {
        match expectation {
            CaseExpectation::SerialContains(expected) => {
                assert_expectations(&report, Some(expected), false, &case.label)?;
            }
            CaseExpectation::MooneyePass => {
                assert_expectations(&report, None, true, &case.label)?;
            }
        }
    }

    Ok(report)
}

fn resolve_case_rom_path(case_path: &Path, suite_path: &Path, rom_root: Option<&Path>) -> PathBuf {
    if case_path.is_absolute() {
        return case_path.to_path_buf();
    }
    if let Some(root) = rom_root {
        return root.join(case_path);
    }
    match suite_path.parent() {
        Some(parent) => parent.join(case_path),
        None => case_path.to_path_buf(),
    }
}

fn parse_suite(content: &str, default_max_steps: usize) -> Result<Vec<SuiteCase>, String> {
    let mut cases = Vec::new();
    for (index, raw_line) in content.lines().enumerate() {
        let line_no = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split('|').map(str::trim).collect();
        if !(2..=4).contains(&parts.len()) {
            return Err(format!(
                "invalid suite line {line_no}: expected 'label|rom_path|[max_steps]|[expectation]'"
            ));
        }

        let label = parts[0];
        let rom_path = parts[1];
        if label.is_empty() || rom_path.is_empty() {
            return Err(format!(
                "invalid suite line {line_no}: label and rom_path are required"
            ));
        }

        let max_steps = if parts.len() >= 3 && !parts[2].is_empty() {
            parts[2].parse::<usize>().map_err(|_| {
                format!("invalid suite line {line_no}: max_steps must be an integer")
            })?
        } else {
            default_max_steps
        };

        let expectation = if parts.len() == 4 && !parts[3].is_empty() {
            Some(
                parse_expectation(parts[3])
                    .map_err(|err| format!("invalid suite line {line_no}: {err}"))?,
            )
        } else {
            None
        };

        cases.push(SuiteCase {
            label: label.to_string(),
            rom_path: PathBuf::from(rom_path),
            max_steps,
            expectation,
        });
    }

    if cases.is_empty() {
        return Err("suite file contains no runnable cases".to_string());
    }

    Ok(cases)
}

fn parse_expectation(raw: &str) -> Result<CaseExpectation, String> {
    if let Some(serial) = raw.strip_prefix("serial:") {
        if serial.is_empty() {
            return Err("serial expectation cannot be empty".to_string());
        }
        return Ok(CaseExpectation::SerialContains(serial.to_string()));
    }

    if raw == "mooneye-pass" {
        return Ok(CaseExpectation::MooneyePass);
    }

    Err("expectation must be 'serial:<text>' or 'mooneye-pass'".to_string())
}

fn run_for_steps(rom_data: &[u8], max_steps: usize) -> Result<ExecutionReport, String> {
    let mut gb = GameBoy::new();
    gb.load_rom(rom_data);
    let mut cycles = 0u64;

    for step in 0..max_steps {
        let step_cycles = gb
            .step()
            .map_err(|err| format!("emulation failed at step {step}: {err}"))?;
        cycles += u64::from(step_cycles);
    }

    let regs = gb.cpu.regs;
    Ok(ExecutionReport {
        steps: max_steps,
        cycles,
        pc: gb.cpu.pc,
        sp: gb.cpu.sp,
        af: regs.af(),
        bc: regs.bc(),
        de: regs.de(),
        hl: regs.hl(),
        serial_output: render_serial(gb.bus.serial_output()),
    })
}

fn assert_expectations(
    report: &ExecutionReport,
    expect_serial: Option<&str>,
    expect_mooneye_pass: bool,
    context: &str,
) -> Result<(), String> {
    if let Some(expected) = expect_serial {
        if !report.serial_output.contains(expected) {
            return Err(format!(
                "{context}: serial expectation failed: expected output containing '{expected}', got '{}'",
                report.serial_output
            ));
        }
    }

    if expect_mooneye_pass && !(report.bc == 0x0305 && report.de == 0x080D && report.hl == 0x1522) {
        return Err(format!(
            "{context}: mooneye pass signature failed: expected BC=0x0305 DE=0x080D HL=0x1522, got BC=0x{:04X} DE=0x{:04X} HL=0x{:04X}",
            report.bc, report.de, report.hl
        ));
    }

    Ok(())
}

fn summarize_serial(serial_output: &str) -> String {
    if serial_output.len() <= 80 {
        return serial_output.to_string();
    }
    format!("{}...", &serial_output[..80])
}

fn render_serial(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "<empty>".to_string();
    }
    bytes
        .iter()
        .map(|byte| {
            if (0x20..=0x7E).contains(byte) || *byte == b'\n' || *byte == b'\r' || *byte == b'\t' {
                char::from(*byte).to_string()
            } else {
                format!("\\x{byte:02X}")
            }
        })
        .collect::<String>()
}

fn render_exec_report(path: &Path, header: &RomHeader, report: &ExecutionReport) -> String {
    format!(
        "ROM: {}\nMode: exec\nTitle: {}\nSteps: {}\nCycles: {}\nPC: 0x{:04X}\nSP: 0x{:04X}\nAF: 0x{:04X}\nBC: 0x{:04X}\nDE: 0x{:04X}\nHL: 0x{:04X}\nSerial Output: {}",
        path.display(),
        header.title,
        report.steps,
        report.cycles,
        report.pc,
        report.sp,
        report.af,
        report.bc,
        report.de,
        report.hl,
        report.serial_output
    )
}

fn render_header(path: &Path, header: &RomHeader) -> String {
    let rom_size = header
        .rom_size_bytes
        .map(|bytes| format!("{} KiB", bytes / 1024))
        .unwrap_or_else(|| format!("unknown code 0x{:02X}", header.rom_size_code));

    let ram_size = header
        .ram_size_bytes
        .map(|bytes| format!("{} KiB", bytes / 1024))
        .unwrap_or_else(|| format!("unknown code 0x{:02X}", header.ram_size_code));

    let licensee = header
        .new_licensee_code
        .as_deref()
        .map(|value| format!("new={value}"))
        .unwrap_or_else(|| format!("old=0x{:02X}", header.old_licensee_code));

    format!(
        "ROM: {}\nTitle: {}\nCGB Mode: {}\nSGB Support: {}\nCartridge: 0x{:02X} ({})\nROM Size: {}\nRAM Size: {}\nDestination Code: 0x{:02X}\nLicensee: {}\nMask ROM Version: {}\nHeader Checksum: 0x{:02X}\nGlobal Checksum: 0x{:04X}",
        path.display(),
        header.title,
        header.cgb_mode,
        if header.sgb_supported { "yes" } else { "no" },
        header.cartridge_type,
        header.cartridge_type_name(),
        rom_size,
        ram_size,
        header.destination_code,
        licensee,
        header.mask_rom_version,
        header.header_checksum,
        header.global_checksum
    )
}

struct ExecutionReport {
    steps: usize,
    cycles: u64,
    pc: u16,
    sp: u16,
    af: u16,
    bc: u16,
    de: u16,
    hl: u16,
    serial_output: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    const NINTENDO_LOGO: [u8; 48] = [
        0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00,
        0x0D, 0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD,
        0xD9, 0x99, 0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 0xDD, 0xDC, 0x99, 0x9F, 0xBB,
        0xB9, 0x33, 0x3E,
    ];

    #[test]
    fn parses_required_rom_argument() {
        let cli = Cli::try_parse_from(["vibegb-runner", "--rom", "Pokemon.gb"])
            .expect("cli parse should succeed");
        assert_eq!(cli.rom, Some(PathBuf::from("Pokemon.gb")));
        assert_eq!(cli.mode, RunnerMode::Header);
        assert_eq!(cli.max_steps, DEFAULT_MAX_STEPS);
    }

    #[test]
    fn supports_short_rom_flag() {
        let cli = Cli::try_parse_from(["vibegb-runner", "-r", "Pokemon.gb"])
            .expect("cli parse should succeed");
        assert_eq!(cli.rom, Some(PathBuf::from("Pokemon.gb")));
    }

    #[test]
    fn prints_header_for_valid_rom() {
        let rom_path = write_rom_with_program("RUNNER TEST", &[]);
        let cli = Cli::try_parse_from([
            "vibegb-runner",
            "--rom",
            rom_path.to_str().expect("path should be utf8"),
        ])
        .expect("cli parse should succeed");

        let output = execute(cli).expect("valid test ROM should load");
        assert!(output.contains("Title: RUNNER TEST"));
        assert!(output.contains("Cartridge: 0x00 (ROM ONLY)"));

        fs::remove_file(rom_path).expect("temp ROM should be removable");
    }

    #[test]
    fn executes_rom_and_matches_serial_expectation() {
        let rom_path = write_rom_with_program("RUN EXEC", &serial_emit_program(b"PASS"));
        let cli = Cli::try_parse_from([
            "vibegb-runner",
            "--rom",
            rom_path.to_str().expect("path should be utf8"),
            "--mode",
            "exec",
            "--max-steps",
            "128",
            "--expect-serial",
            "PASS",
        ])
        .expect("cli parse should succeed");

        let output = execute(cli).expect("execution should succeed");
        assert!(output.contains("Mode: exec"));
        assert!(output.contains("Serial Output: PASS"));

        fs::remove_file(rom_path).expect("temp ROM should be removable");
    }

    #[test]
    fn executes_rom_and_matches_mooneye_signature() {
        let rom_path = write_rom_with_program("MOONEYE", &mooneye_pass_program());
        let cli = Cli::try_parse_from([
            "vibegb-runner",
            "--rom",
            rom_path.to_str().expect("path should be utf8"),
            "--mode",
            "exec",
            "--max-steps",
            "64",
            "--expect-mooneye-pass",
        ])
        .expect("cli parse should succeed");

        let output = execute(cli).expect("execution should satisfy mooneye signature");
        assert!(output.contains("BC: 0x0305"));
        assert!(output.contains("DE: 0x080D"));
        assert!(output.contains("HL: 0x1522"));

        fs::remove_file(rom_path).expect("temp ROM should be removable");
    }

    #[test]
    fn reports_error_when_serial_expectation_fails() {
        let rom_path = write_rom_with_program("RUN EXEC", &serial_emit_program(b"PASS"));
        let cli = Cli::try_parse_from([
            "vibegb-runner",
            "--rom",
            rom_path.to_str().expect("path should be utf8"),
            "--mode",
            "exec",
            "--max-steps",
            "128",
            "--expect-serial",
            "FAIL",
        ])
        .expect("cli parse should succeed");

        let err = execute(cli).expect_err("mismatched serial expectation should fail");
        assert!(err.contains("serial expectation failed"));

        fs::remove_file(rom_path).expect("temp ROM should be removable");
    }

    #[test]
    fn parses_suite_lines_with_defaults_and_expectations() {
        let suite = "\
# comment
cpu-01|blargg/cpu01.gb|2000|serial:Passed
timer-01|mooneye/timer.gb||mooneye-pass
header-only|misc/smoke.gb
";
        let parsed = parse_suite(suite, 555).expect("suite should parse");
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].label, "cpu-01");
        assert_eq!(parsed[0].max_steps, 2000);
        assert_eq!(
            parsed[0].expectation,
            Some(CaseExpectation::SerialContains("Passed".to_string()))
        );
        assert_eq!(parsed[1].max_steps, 555);
        assert_eq!(parsed[1].expectation, Some(CaseExpectation::MooneyePass));
        assert_eq!(parsed[2].max_steps, 555);
        assert_eq!(parsed[2].expectation, None);
    }

    #[test]
    fn rejects_invalid_suite_expectation() {
        let suite = "bad|rom.gb|100|unknown";
        let err = parse_suite(suite, 1000).expect_err("should reject unknown expectation");
        assert!(err.contains("expectation must be 'serial:<text>' or 'mooneye-pass'"));
    }

    #[test]
    fn executes_suite_and_reports_failure_summary() {
        let root = temp_dir("suite-root");
        fs::create_dir_all(&root).expect("suite root dir should exist");

        let serial_rom = root.join("serial-pass.gb");
        let mooneye_rom = root.join("mooneye-pass.gb");
        write_rom_file(&serial_rom, "SERIAL", &serial_emit_program(b"Passed"));
        write_rom_file(&mooneye_rom, "MOONEYE", &mooneye_pass_program());

        let suite_path = root.join("m1-suite.txt");
        let suite = "\
serial-case|serial-pass.gb|256|serial:Passed
mooneye-case|mooneye-pass.gb|64|mooneye-pass
failing-case|serial-pass.gb|256|serial:FAIL
";
        fs::write(&suite_path, suite).expect("suite file should be written");

        let err = execute_suite(&suite_path, None, DEFAULT_MAX_STEPS)
            .expect_err("suite should fail due to one failing case");
        assert!(err.contains("Summary: total=3 passed=2 failed=1"));
        assert!(err.contains("PASS | serial-case"));
        assert!(err.contains("PASS | mooneye-case"));
        assert!(err.contains("FAIL | failing-case"));

        fs::remove_file(&suite_path).expect("suite should be removable");
        fs::remove_file(&serial_rom).expect("serial rom should be removable");
        fs::remove_file(&mooneye_rom).expect("mooneye rom should be removable");
        fs::remove_dir_all(&root).expect("suite root should be removable");
    }

    #[test]
    fn suite_uses_rom_root_for_relative_paths() {
        let suite_root = temp_dir("suite-root");
        let rom_root = temp_dir("rom-root");
        fs::create_dir_all(&suite_root).expect("suite root dir should exist");
        fs::create_dir_all(&rom_root).expect("rom root dir should exist");

        let rom_path = rom_root.join("serial-pass.gb");
        write_rom_file(&rom_path, "SERIAL", &serial_emit_program(b"Passed"));

        let suite_path = suite_root.join("m1-suite.txt");
        fs::write(
            &suite_path,
            "serial-case|serial-pass.gb|256|serial:Passed\n",
        )
        .expect("suite file should be written");

        let output = execute_suite(&suite_path, Some(&rom_root), DEFAULT_MAX_STEPS)
            .expect("suite should pass with explicit rom root");
        assert!(output.contains("Summary: total=1 passed=1 failed=0"));

        fs::remove_file(&suite_path).expect("suite should be removable");
        fs::remove_file(&rom_path).expect("rom should be removable");
        fs::remove_dir_all(&suite_root).expect("suite root should be removable");
        fs::remove_dir_all(&rom_root).expect("rom root should be removable");
    }

    fn serial_emit_program(text: &[u8]) -> Vec<u8> {
        let mut program = Vec::with_capacity((text.len() * 10) + 2);
        for byte in text {
            program.extend_from_slice(&[0x3E, *byte]); // LD A, d8
            program.extend_from_slice(&[0xEA, 0x01, 0xFF]); // LD (FF01), A
            program.extend_from_slice(&[0x3E, 0x81]); // LD A, 0x81
            program.extend_from_slice(&[0xEA, 0x02, 0xFF]); // LD (FF02), A
        }
        program.extend_from_slice(&[0x18, 0xFE]); // JR -2
        program
    }

    fn mooneye_pass_program() -> Vec<u8> {
        vec![
            0x06, 0x03, // LD B,03
            0x0E, 0x05, // LD C,05
            0x16, 0x08, // LD D,08
            0x1E, 0x0D, // LD E,0D
            0x26, 0x15, // LD H,15
            0x2E, 0x22, // LD L,22
            0x18, 0xFE, // JR -2
        ]
    }

    fn write_rom_with_program(title: &str, program: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "vibegb-runner-test-{}-{}.gb",
            title,
            unique_suffix()
        ));
        write_rom_file(&path, title, program);
        path
    }

    fn write_rom_file(path: &Path, title: &str, program: &[u8]) {
        let mut rom = vec![0; 0x8000];
        rom[0x100..0x104].copy_from_slice(&[0xC3, 0x50, 0x01, 0x00]); // JP 0x0150
        rom[0x104..0x134].copy_from_slice(&NINTENDO_LOGO);

        let title_bytes = title.as_bytes();
        let title_len = title_bytes.len().min(16);
        rom[0x134..0x134 + title_len].copy_from_slice(&title_bytes[..title_len]);

        rom[0x143] = 0x00;
        rom[0x146] = 0x00;
        rom[0x147] = 0x00;
        rom[0x148] = 0x00;
        rom[0x149] = 0x00;
        rom[0x14A] = 0x01;
        rom[0x14B] = 0x01;
        rom[0x14C] = 0x00;

        let program_start = 0x150;
        let max_program_len = rom.len() - program_start;
        let program_len = program.len().min(max_program_len);
        rom[program_start..program_start + program_len].copy_from_slice(&program[..program_len]);

        rom[0x14D] = calculate_header_checksum(&rom);
        rom[0x14E] = 0xAB;
        rom[0x14F] = 0xCD;

        fs::write(path, rom).expect("temp ROM should be written");
    }

    fn temp_dir(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!("vibegb-{prefix}-{}", unique_suffix()))
    }

    fn unique_suffix() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos()
    }

    fn calculate_header_checksum(data: &[u8]) -> u8 {
        let mut checksum = 0u8;
        for byte in &data[0x134..=0x14C] {
            checksum = checksum.wrapping_sub(*byte).wrapping_sub(1);
        }
        checksum
    }
}
