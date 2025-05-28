// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

// Common types and statics that are used across platforms
pub type Movement = (i32, i32);
static RECORDING: AtomicBool = AtomicBool::new(false);
static PLAYBACK: AtomicBool = AtomicBool::new(false);
static TO_RECORD: AtomicBool = AtomicBool::new(false);
static TO_PLAYBACK: AtomicBool = AtomicBool::new(false);
static mut RECORDED_MOVEMENTS: Mutex<Vec<(i32, i32)>> = Mutex::new(Vec::new());

pub static KEY_RECORD: i32 = 0x54;
pub static KEY_PLAY: i32 = 0x50;
pub static KEY_PAUSE: i32 = 0x20;
pub static KEY_STOP: i32 = 0x53;

// Platform-specific implementations
#[cfg(target_os = "windows")]
mod windows {
    use super::*;
    use std::mem::*;
    use std::ptr::*;
    use std::time::{Duration, Instant};
    use winapi::ctypes::c_int;
    use winapi::shared::minwindef::{LPVOID, UINT};
    use winapi::shared::ntdef::NULL;
    use winapi::um::winuser::*;

    pub unsafe fn record() -> Vec<Movement> {
        let mut moves = Vec::new();

        // Create a message-only window
        let window_class: Vec<u16> = "Static".encode_utf16().chain(std::iter::once(0)).collect();
        let hwnd = CreateWindowExW(
            0,
            window_class.as_ptr(),
            null(),
            0,
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            null_mut(),
            null_mut(),
            null_mut(),
        );

        let rid = RAWINPUTDEVICE {
            usUsagePage: 0x01,
            usUsage: 0x02,
            dwFlags: RIDEV_INPUTSINK,
            hwndTarget: hwnd,
        }; // 0x1 for mouse; "RIDEV_INPUTSINK" to get inputs even when not in focus

        RegisterRawInputDevices(
            &rid as *const RAWINPUTDEVICE,
            1,
            size_of::<RAWINPUTDEVICE>() as UINT,
        );

        let start = Instant::now(); // get the current time so easily (W rust)
        let mut last_pos = (0i32, 0i32); // last position in tuple instead of std::pair<>
        let mut i = 0;

        while RECORDING.load(Ordering::Relaxed) {
            // fuck AtomicBool in rust
            let mut msg: MSG = zeroed(); // std::mem::zeroed() cz I can't make a variable without giving it a value for some fucking reason
            while PeekMessageW(&mut msg, hwnd, 0, 0, PM_REMOVE) != 0 {
                // check if there are any messages; PM_REMOVE to remove them after viewing
                if msg.message == WM_INPUT {
                    // check if the message is a raw input message
                    let raw: RAWINPUT = zeroed();
                    let mut size = size_of_val(&raw) as UINT; // size needs to be type UINT bc Win32 API, so I force it

                    GetRawInputData(
                        msg.lParam as HRAWINPUT, // Win32 API types are stupid in rust, none of the parameter vars are what they would be in C++.
                        RID_INPUT,
                        &raw as *const RAWINPUT as LPVOID, // I have to dereference the reference to raw and cast it to LPVOID because that's what the function takes in, took me a while to understand this, but it makes sense
                        &mut size,
                        size_of::<RAWINPUTHEADER>() as UINT, // this is apparently how you use normal size_of() func in rust for some fucking reason
                    );

                    // add all the input events in the past millisecond to the vector instead of adding EVERY input event to the vector
                    // this SIGNIFICANTLY reduces memory usage and file size once we add that
                    last_pos.0 += raw.data.mouse().lLastX;
                    last_pos.1 += raw.data.mouse().lLastY;
                }
            }

            // add the last pos to the vector and reset the value for next millisecond
            moves.push(last_pos);
            last_pos = (0, 0);

            i += 1; // Rust has no ++ operator </3
            thread::sleep((start + Duration::from_millis(i)) - Instant::now()); // ads 1 millisecond to the current time and sleeps until then
        }

        // Cleanup
        DestroyWindow(hwnd);

        moves
    }

    pub unsafe fn replay(moves: Movement) {
        if !PLAYBACK.load(Ordering::Relaxed) {
            // if playback is not true, exit;
            return;
        }

        let (dx, dy) = moves; // "unpack" the current "moves" tuple into 2 values
        if moves == (0, 0) {
            return;
        }

        println!("Replaying movement: ({}, {})", dx, dy);

        // Creates the mouse event to send by making a MOUSEINPUT struct and using it to make the final INPUT struct to send out
        let movement = &MOUSEINPUT {
            dx,
            dy,
            mouseData: 0,
            dwFlags: MOUSEEVENTF_MOVE,
            time: 0,
            dwExtraInfo: 0,
        };
        let mut input = INPUT {
            type_: INPUT_MOUSE,
            u: transmute_copy(movement),
        }; // transmute_copy() casts it to a INPUT_u

        SendInput(1, &mut input, size_of_val(&input) as c_int); // Send 1 input, "input", with the size of "input"
    }

    pub unsafe fn move_mouse() {
        if (GetAsyncKeyState(KEY_RECORD) & 0x0001) != 0 || TO_RECORD.load(Ordering::Relaxed) {
            // check if T gets pressed or TO_RECORD is true
            if !RECORDING.load(Ordering::Relaxed) {
                // check if RECORDING is !true (AtomicBool)
                println!("Recording...");
                RECORDING.store(true, Ordering::Relaxed); // Set RECORDING to true
                TO_RECORD.store(false, Ordering::Relaxed); // Set TO_RECORD to false
                thread::spawn(|| {
                    let recorded_moves = record();
                    //for mover in &recorded_moves {
                    //    println!("{}", mover.0);
                    //    println!("{}", mover.1);
                    //}

                    RECORDING.store(false, Ordering::Relaxed); // Set RECORDING to false
                    let mut moves = RECORDED_MOVEMENTS.lock().unwrap();
                    *moves = recorded_moves;

                    println!("Finished.");
                });
            } else {
                RECORDING.store(false, Ordering::Relaxed);
                TO_RECORD.store(false, Ordering::Relaxed); // Set TO_RECORD to false
                TO_PLAYBACK.store(false, Ordering::Relaxed); // Set TO_PLAYBACK to false
            }
        } else if (GetAsyncKeyState(0x50) & 0x0001) != 0 || TO_PLAYBACK.load(Ordering::Relaxed) {
            // check if P gets pressed or TO_PLAYBACK is true
            let moves = RECORDED_MOVEMENTS.lock().unwrap();
            if !moves.is_empty() && !PLAYBACK.load(Ordering::Relaxed) {
                // if not playing back and there are recorded moves
                println!("Replaying...");
                PLAYBACK.store(true, Ordering::Relaxed); // Set PLAYBACK to true

                let moves_copy = moves.clone(); // clone the moves for type safety (also rust won't let me directly reference moves inside the thread)
                thread::spawn(move || {
                    // move because rust makes me
                    let start = Instant::now();
                    for (i, &movement) in moves_copy.iter().enumerate() {
                        if !PLAYBACK.load(Ordering::Relaxed) {
                            break;
                        }

                        replay(movement);

                        let target_time = start + Duration::from_millis((i + 1) as u64);
                        if let Some(sleep_duration) = target_time.checked_duration_since(Instant::now())
                        {
                            thread::sleep(sleep_duration);
                        }
                    }
                    PLAYBACK.store(false, Ordering::Relaxed); // Set PLAYBACK to false
                    TO_PLAYBACK.store(false, Ordering::Relaxed); // Set TO_PLAYBACK to false
                    println!("Finished.");
                });
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;

    pub unsafe fn record() -> Vec<Movement> {
        println!("Recording not implemented for macOS yet");
        Vec::new()
    }

    pub unsafe fn replay(moves: Movement) {
        println!("Replay not implemented for macOS yet");
    }

    pub unsafe fn move_mouse() {
        println!("Mouse movement not implemented for macOS yet");
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;

    pub unsafe fn record() -> Vec<Movement> {
        println!("Recording not implemented for Linux yet");
        Vec::new()
    }

    pub unsafe fn replay(moves: Movement) {
        println!("Replay not implemented for Linux yet");
    }

    pub unsafe fn move_mouse() {
        println!("Mouse movement not implemented for Linux yet");
    }
}

// Platform-specific function selection
#[cfg(target_os = "windows")]
use windows::{record, replay, move_mouse};
#[cfg(target_os = "macos")]
use macos::{record, replay, move_mouse};
#[cfg(target_os = "linux")]
use linux::{record, replay, move_mouse};

#[tauri::command]
fn setup() {
    println!("Press T to start/stop recording, Press P to replay");
    RECORDING.store(false, Ordering::Relaxed);
    thread::spawn(|| loop {
        unsafe {
            move_mouse();
        }
        thread::sleep(Duration::from_millis(50));
    });
}

#[tauri::command]
fn set_record() {
    TO_RECORD.store(true, Ordering::Relaxed);
}

#[tauri::command]
fn set_replay() {
    TO_PLAYBACK.store(true, Ordering::Relaxed);
}

// #[tauri::command]
// fn rebind(key: i32) {
//     if new != *key {
//         *key = new;
//     }
// }

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    setup();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![setup, set_record, set_replay])
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
