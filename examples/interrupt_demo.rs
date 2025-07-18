use sage_core::interrupt::{
    global_interrupt_manager, reset_global_interrupt_manager, 
    interrupt_current_task, InterruptReason
};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Sage Agent Interrupt Demo");
    println!("============================");
    
    // Reset the global interrupt manager
    reset_global_interrupt_manager();
    
    // Demo 1: Basic interrupt functionality
    println!("\n📋 Demo 1: Basic Interrupt Functionality");
    println!("Creating a task scope...");
    
    let task_scope = global_interrupt_manager()
        .lock()
        .unwrap()
        .create_task_scope();
    
    println!("Task scope created. Checking if cancelled: {}", task_scope.is_cancelled());
    
    // Simulate interrupting the task
    println!("Interrupting the task...");
    interrupt_current_task(InterruptReason::UserInterrupt);
    
    // Give a moment for cancellation to propagate
    sleep(Duration::from_millis(10)).await;
    
    println!("Task scope cancelled: {}", task_scope.is_cancelled());
    
    // Demo 2: Using select! with cancellation
    println!("\n📋 Demo 2: Select! with Cancellation");
    reset_global_interrupt_manager();
    
    let manager = global_interrupt_manager().lock().unwrap().clone();
    let token = manager.cancellation_token();
    
    println!("Starting a long-running task...");
    
    // Spawn a task that simulates interruption after 1 second
    tokio::spawn(async move {
        sleep(Duration::from_millis(500)).await;
        println!("⚡ Simulating Ctrl+C interrupt...");
        interrupt_current_task(InterruptReason::UserInterrupt);
    });
    
    let result = tokio::select! {
        _ = sleep(Duration::from_secs(5)) => {
            "Task completed normally"
        }
        _ = token.cancelled() => {
            "Task was interrupted!"
        }
    };
    
    println!("Result: {}", result);
    
    // Demo 3: Multiple task scopes
    println!("\n📋 Demo 3: Multiple Task Scopes");
    reset_global_interrupt_manager();
    
    let manager = global_interrupt_manager().lock().unwrap();
    let scope1 = manager.create_task_scope();
    let scope2 = manager.create_task_scope();
    let scope3 = manager.create_task_scope();
    drop(manager);
    
    println!("Created 3 task scopes");
    println!("Scope 1 cancelled: {}", scope1.is_cancelled());
    println!("Scope 2 cancelled: {}", scope2.is_cancelled());
    println!("Scope 3 cancelled: {}", scope3.is_cancelled());
    
    println!("Interrupting all tasks...");
    interrupt_current_task(InterruptReason::UserInterrupt);
    
    // Give a moment for cancellation to propagate
    sleep(Duration::from_millis(10)).await;
    
    println!("After interrupt:");
    println!("Scope 1 cancelled: {}", scope1.is_cancelled());
    println!("Scope 2 cancelled: {}", scope2.is_cancelled());
    println!("Scope 3 cancelled: {}", scope3.is_cancelled());
    
    // Demo 4: Interrupt reasons
    println!("\n📋 Demo 4: Different Interrupt Reasons");
    reset_global_interrupt_manager();
    
    let manager = global_interrupt_manager().lock().unwrap();
    let _receiver = manager.subscribe();
    drop(manager);
    
    // Test different interrupt reasons
    let reasons = vec![
        InterruptReason::UserInterrupt,
        InterruptReason::Timeout,
        InterruptReason::Shutdown,
        InterruptReason::Manual,
    ];
    
    for reason in reasons {
        reset_global_interrupt_manager();
        let manager = global_interrupt_manager().lock().unwrap();
        let mut receiver = manager.subscribe();
        drop(manager);
        
        println!("Testing interrupt reason: {:?}", reason);
        interrupt_current_task(reason.clone());
        
        if let Ok(received_reason) = receiver.try_recv() {
            println!("Received reason: {:?}", received_reason);
            assert_eq!(received_reason, reason);
        }
    }
    
    println!("\n✅ All demos completed successfully!");
    println!("The interrupt system is working correctly.");
    
    Ok(())
}
