//! Integration tests for the CCPlayer media player
//!
//! These tests verify the complete player functionality including:
//! - File loading and playback
//! - Play/pause/seek operations
//! - Error handling
//! - Resource cleanup

use anyhow::Result;
use ccplayer::player::{MediaPlayer, PlayerState};
use ccplayer::decoder::StreamInfo;
use ccplayer_integration_tests::{TestFixture, perf_test::PerfMeasure};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_player_initialization() -> Result<()> {
    // Test that the player can be created and initialized properly
    let player = MediaPlayer::new()?;
    
    assert_eq!(player.state(), PlayerState::Idle);
    assert_eq!(player.position(), Duration::ZERO);
    assert_eq!(player.duration(), Duration::ZERO);
    
    Ok(())
}

#[tokio::test]
async fn test_load_video_file() -> Result<()> {
    let fixture = TestFixture::new()?;
    let mut player = MediaPlayer::new()?;
    
    // Load a test video file
    player.load_file(&fixture.media_files.video_h264).await?;
    
    // Verify the player state after loading
    assert_eq!(player.state(), PlayerState::Ready);
    
    // Check that stream info is available
    let info = player.stream_info().expect("Stream info should be available");
    assert!(info.has_video());
    assert!(info.duration() > Duration::ZERO);
    
    Ok(())
}

#[tokio::test]
async fn test_play_pause_operations() -> Result<()> {
    let fixture = TestFixture::new()?;
    let mut player = MediaPlayer::new()?;
    
    // Load and play
    player.load_file(&fixture.media_files.video_h264).await?;
    player.play()?;
    
    assert_eq!(player.state(), PlayerState::Playing);
    
    // Let it play for a short time
    sleep(Duration::from_millis(100)).await;
    
    // Pause
    player.pause()?;
    assert_eq!(player.state(), PlayerState::Paused);
    
    let position_at_pause = player.position();
    
    // Wait and verify position doesn't change while paused
    sleep(Duration::from_millis(100)).await;
    assert_eq!(player.position(), position_at_pause);
    
    // Resume
    player.play()?;
    assert_eq!(player.state(), PlayerState::Playing);
    
    // Verify position advances after resuming
    sleep(Duration::from_millis(100)).await;
    assert!(player.position() > position_at_pause);
    
    Ok(())
}

#[tokio::test]
async fn test_seek_operation() -> Result<()> {
    let fixture = TestFixture::new()?;
    let mut player = MediaPlayer::new()?;
    
    player.load_file(&fixture.media_files.video_h264).await?;
    player.play()?;
    
    // Seek to middle of the video
    let duration = player.duration();
    let seek_target = duration / 2;
    
    player.seek(seek_target)?;
    
    // Allow some time for seek to complete
    sleep(Duration::from_millis(200)).await;
    
    // Position should be close to seek target (within 100ms)
    let position = player.position();
    assert!((position.as_millis() as i64 - seek_target.as_millis() as i64).abs() < 100);
    
    Ok(())
}

#[tokio::test]
async fn test_volume_control() -> Result<()> {
    let mut player = MediaPlayer::new()?;
    
    // Test volume range
    assert_eq!(player.volume(), 1.0); // Default volume
    
    player.set_volume(0.5)?;
    assert!((player.volume() - 0.5).abs() < 0.01);
    
    player.set_volume(0.0)?;
    assert_eq!(player.volume(), 0.0);
    
    player.set_volume(1.0)?;
    assert_eq!(player.volume(), 1.0);
    
    // Test out of range values are clamped
    player.set_volume(1.5)?;
    assert_eq!(player.volume(), 1.0);
    
    player.set_volume(-0.5)?;
    assert_eq!(player.volume(), 0.0);
    
    Ok(())
}

#[tokio::test]
async fn test_load_multiple_formats() -> Result<()> {
    let fixture = TestFixture::new()?;
    let mut player = MediaPlayer::new()?;
    
    // Test loading different video formats
    let formats = vec![
        (&fixture.media_files.video_h264, "H.264"),
        (&fixture.media_files.video_h265, "H.265"),
    ];
    
    for (path, format_name) in formats {
        player.load_file(path).await?;
        assert_eq!(player.state(), PlayerState::Ready, 
                   "Failed to load {} format", format_name);
        
        // Verify we can play each format
        player.play()?;
        sleep(Duration::from_millis(50)).await;
        player.stop()?;
    }
    
    Ok(())
}

#[tokio::test]
async fn test_playback_completion() -> Result<()> {
    let fixture = TestFixture::new()?;
    let mut player = MediaPlayer::new()?;
    
    // Create a very short test file
    let short_video = fixture.path().join("short_video.mp4");
    std::fs::write(&short_video, b"short_video_data")?;
    
    player.load_file(&short_video).await?;
    player.play()?;
    
    // Wait for playback to complete
    let mut completion_detected = false;
    for _ in 0..50 { // 5 seconds timeout
        if player.state() == PlayerState::Ended {
            completion_detected = true;
            break;
        }
        sleep(Duration::from_millis(100)).await;
    }
    
    assert!(completion_detected, "Playback completion was not detected");
    
    Ok(())
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let mut player = MediaPlayer::new()?;
    
    // Test loading non-existent file
    let result = player.load_file(&std::path::PathBuf::from("/non/existent/file.mp4")).await;
    assert!(result.is_err());
    assert_eq!(player.state(), PlayerState::Error);
    
    // Test operations on unloaded player
    let mut player = MediaPlayer::new()?;
    assert!(player.play().is_err());
    assert!(player.seek(Duration::from_secs(10)).is_err());
    
    Ok(())
}

#[tokio::test]
#[cfg_attr(not(feature = "stress-tests"), ignore)]
async fn test_stress_rapid_seek() -> Result<()> {
    let fixture = TestFixture::new()?;
    let mut player = MediaPlayer::new()?;
    
    player.load_file(&fixture.media_files.video_h264).await?;
    player.play()?;
    
    let duration = player.duration();
    
    // Perform rapid seeks
    for i in 0..20 {
        let position = duration * i / 20;
        player.seek(position)?;
        sleep(Duration::from_millis(50)).await;
    }
    
    // Player should still be in a valid state
    assert!(matches!(player.state(), PlayerState::Playing | PlayerState::Buffering));
    
    Ok(())
}

#[tokio::test]
async fn test_performance_metrics() -> Result<()> {
    let fixture = TestFixture::new()?;
    let mut player = MediaPlayer::new()?;
    
    let mut load_perf = PerfMeasure::new("File Loading");
    let mut seek_perf = PerfMeasure::new("Seek Operation");
    
    // Measure file loading time
    load_perf.start();
    player.load_file(&fixture.media_files.video_h264).await?;
    load_perf.stop();
    
    player.play()?;
    let duration = player.duration();
    
    // Measure seek times
    for i in 1..=5 {
        let position = duration * i / 6;
        seek_perf.start();
        player.seek(position)?;
        seek_perf.stop();
        sleep(Duration::from_millis(100)).await;
    }
    
    // Report performance
    load_perf.report();
    seek_perf.report();
    
    // Verify performance targets
    assert!(load_perf.average() < Duration::from_millis(500), 
            "File loading too slow");
    assert!(seek_perf.average() < Duration::from_millis(100), 
            "Seek operation too slow");
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    use tokio::task;
    
    let fixture = TestFixture::new()?;
    let player = std::sync::Arc::new(tokio::sync::Mutex::new(MediaPlayer::new()?));
    
    // Load file
    {
        let mut p = player.lock().await;
        p.load_file(&fixture.media_files.video_h264).await?;
        p.play()?;
    }
    
    // Spawn concurrent tasks
    let player1 = player.clone();
    let volume_task = task::spawn(async move {
        for i in 0..10 {
            let volume = i as f32 / 10.0;
            let mut p = player1.lock().await;
            p.set_volume(volume).ok();
            drop(p);
            sleep(Duration::from_millis(50)).await;
        }
    });
    
    let player2 = player.clone();
    let position_task = task::spawn(async move {
        let mut positions = Vec::new();
        for _ in 0..10 {
            let p = player2.lock().await;
            positions.push(p.position());
            drop(p);
            sleep(Duration::from_millis(50)).await;
        }
        positions
    });
    
    // Wait for tasks to complete
    volume_task.await?;
    let positions = position_task.await?;
    
    // Verify positions are monotonically increasing
    for window in positions.windows(2) {
        assert!(window[1] >= window[0], "Playback position went backwards");
    }
    
    Ok(())
}

#[tokio::test]
async fn test_resource_cleanup() -> Result<()> {
    let fixture = TestFixture::new()?;
    
    // Create and destroy multiple players
    for _ in 0..5 {
        let mut player = MediaPlayer::new()?;
        player.load_file(&fixture.media_files.video_h264).await?;
        player.play()?;
        sleep(Duration::from_millis(100)).await;
        player.stop()?;
        // Player dropped here
    }
    
    // If we get here without crashes or leaks, cleanup is working
    Ok(())
}