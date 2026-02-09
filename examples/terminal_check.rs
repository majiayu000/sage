//! Terminal detection diagnostic
//! Run: cargo run --example terminal_check

use crossterm::terminal;

fn main() {
    println!("=== Terminal Detection Diagnostic ===\n");

    // Check various terminal size methods
    let crossterm_size = terminal::size();
    println!("1. crossterm::terminal::size():");
    match crossterm_size {
        Ok((w, h)) => println!("   Width: {}, Height: {}", w, h),
        Err(ref e) => println!("   Error: {}", e),
    }

    // Check environment variables
    println!("\n2. Environment variables:");
    println!("   COLUMNS: {:?}", std::env::var("COLUMNS").ok());
    println!("   LINES: {:?}", std::env::var("LINES").ok());
    println!("   TERM: {:?}", std::env::var("TERM").ok());
    println!("   COLORTERM: {:?}", std::env::var("COLORTERM").ok());
    println!("   TERM_PROGRAM: {:?}", std::env::var("TERM_PROGRAM").ok());

    // Check if we're in a TTY
    println!("\n3. TTY check:");
    println!("   (skipped - requires libc)");

    // Print a ruler to visually check width
    println!("\n4. Visual ruler (every 10 chars):");
    let ruler: String = (1..=20).map(|n| format!("{:<10}", n * 10)).collect();
    println!("   {}", ruler);
    let ticks: String = (0..200)
        .map(|i| if i % 10 == 0 { '|' } else { '-' })
        .collect();
    println!("   {}", &ticks[..ticks.len().min(200)]);

    // Test what sage sees
    println!("\n5. Sage terminal size detection:");
    let (sage_width, sage_height) = terminal::size().unwrap_or((80, 24));
    println!("   sage uses: {}x{}", sage_width, sage_height);

    // Recommendation
    println!("\n=== Recommendation ===");
    if let Ok((w, _)) = crossterm_size {
        if w > 150 {
            println!("Large terminal width detected ({}). This is fine.", w);
            println!("If UI is misaligned, the issue is in the layout code.");
        } else if w < 60 {
            println!(
                "Small terminal width detected ({}). Try widening your terminal.",
                w
            );
        } else {
            println!("Normal terminal width ({}).", w);
        }
    }
}
