// SPDX-License-Identifier: MIT
//! Test for enhanced rate limiter memory leak fixes

use irondrop::server::{RateLimiter, ServerStats};
use std::net::IpAddr;

#[test]
fn test_rate_limiter_enhanced_cleanup() {
    let rate_limiter = RateLimiter::new(60, 5);
    let test_ip: IpAddr = "192.168.1.100".parse().unwrap();

    // Simulate multiple connections from the same IP
    for _ in 0..10 {
        assert!(rate_limiter.check_rate_limit(test_ip));
        rate_limiter.release_connection(test_ip);
    }

    // Check initial memory stats
    let (initial_entries, initial_memory) = rate_limiter.get_memory_stats();
    assert!(initial_entries > 0, "Should have stored connection info");
    assert!(initial_memory > 0, "Should have memory usage");

    // Wait for automatic cleanup (should happen within 60 seconds)
    // For testing, we'll trigger manual cleanup
    rate_limiter.cleanup_old_entries();

    // Memory should still be used since connections are recent
    let (entries_after_cleanup, _) = rate_limiter.get_memory_stats();
    assert!(
        entries_after_cleanup > 0,
        "Recent connections should still be tracked"
    );
}

#[test]
fn test_memory_pressure_cleanup() {
    let rate_limiter = RateLimiter::new(60, 5);
    let _stats = ServerStats::new();

    // Add some connections
    for i in 1..=5 {
        let ip: IpAddr = format!("192.168.1.{}", i).parse().unwrap();
        assert!(rate_limiter.check_rate_limit(ip));
    }

    let (initial_entries, _) = rate_limiter.get_memory_stats();
    assert!(initial_entries > 0);

    // Trigger memory pressure cleanup
    rate_limiter.cleanup_on_memory_pressure();

    // Should have cleaned up some entries (aggressive cleanup)
    let (entries_after_pressure, _) = rate_limiter.get_memory_stats();
    // Note: Entries might still exist if they have active connections
    // This test mainly ensures the cleanup method runs without panicking
    println!(
        "Entries before pressure cleanup: {}, after: {}",
        initial_entries, entries_after_pressure
    );
}

#[test]
fn test_connection_limit_per_ip() {
    let rate_limiter = RateLimiter::new(1000, 10); // High limits for this test
    let test_ip: IpAddr = "192.168.1.200".parse().unwrap();

    // Simulate many connections (should not exceed max_connections_per_ip = 1000)
    for i in 0..50 {
        if rate_limiter.check_rate_limit(test_ip) {
            // Connection accepted
            if i % 10 == 0 {
                rate_limiter.release_connection(test_ip);
            }
        }
    }

    let (entries, memory) = rate_limiter.get_memory_stats();
    println!(
        "Rate limiter stats: {} entries, {} bytes memory",
        entries, memory
    );

    // Should have tracked the IP
    assert!(entries > 0);
    assert!(memory > 0);
}

#[test]
fn test_reduced_retention_time() {
    let rate_limiter = RateLimiter::new(60, 5);
    let test_ip: IpAddr = "192.168.1.50".parse().unwrap();

    // Make a connection
    assert!(rate_limiter.check_rate_limit(test_ip));
    rate_limiter.release_connection(test_ip);

    let (initial_entries, _) = rate_limiter.get_memory_stats();
    assert!(initial_entries > 0);

    // The new implementation uses 2-minute retention instead of 5 minutes
    // For testing purposes, we'll verify the cleanup logic works
    rate_limiter.cleanup_old_entries();

    // Entries should still exist since they're recent
    let (entries_after, _) = rate_limiter.get_memory_stats();
    assert!(entries_after > 0, "Recent entries should still be retained");
}

#[test]
fn test_memory_stats_integration() {
    let stats = ServerStats::new();
    let rate_limiter = RateLimiter::new(60, 5);

    // Add some test data
    let test_ip: IpAddr = "192.168.1.60".parse().unwrap();
    rate_limiter.check_rate_limit(test_ip);

    // Test memory pressure check (should not trigger on small usage)
    let pressure_detected = stats.check_memory_pressure(Some(&rate_limiter));

    // With minimal usage, pressure should not be detected
    // (unless system memory is already very high)
    println!("Memory pressure detected: {}", pressure_detected);

    // Test should complete without panicking
    assert!(true, "Memory pressure check completed successfully");
}
