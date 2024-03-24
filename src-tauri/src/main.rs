// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{cell::{Ref, RefCell}, collections::HashMap, fs::File, io::BufReader, sync::Mutex};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use tauri::{InvokeError, Manager, State, Wry};
use rodio::{Sink, Decoder, OutputStream, source::{Buffered, Source}};
use anyhow::Result;
use tauri_plugin_store::{Store, StoreBuilder};


type BufferedSound = Buffered<Decoder<BufReader<File>>>;

#[derive(Serialize, Deserialize, Clone)]
struct SoundInfo {
    name: String,
    path: String
}

struct Sound {
    name: String,
    path: String,
    // TODO: make this an Option<> so we know when it fails to load
    sound: BufferedSound,
}

impl Into<SoundInfo> for Sound {
    fn into(self) -> SoundInfo {
        SoundInfo {
            name: self.name,
            path: self.path
        }
    }
}

impl From<&Sound> for SoundInfo {
    fn from(sound: &Sound) -> SoundInfo {
        SoundInfo {
            name: sound.name.clone(),
            path: sound.path.clone()
        }
    }
}

impl From<Ref<'_, Sound>> for SoundInfo {
    fn from(sound: Ref<Sound>) -> SoundInfo {
        SoundInfo {
            name: sound.name.clone(),
            path: sound.path.clone()
        }
    }
}

impl Sound {
    fn from_path(name: String, path: String) -> Result<Self> {
        let file = File::open(&path)?;
        let sound = Decoder::new(BufReader::new(file))?.buffered();

        Ok(Sound {
            name,
            path,
            sound
        })
    }

    fn from_soundinfo(sound_info: SoundInfo) -> Result<Self> {
        Self::from_path(sound_info.name, sound_info.path)
    }

}

struct AppData {
    sink: Sink,
    sounds_name: Mutex<HashMap<String, RefCell<Sound>>>
}

impl AppData {
    pub fn try_default(sink: Sink) -> Result<Self> {
        Ok(AppData {
            sink,
            sounds_name: Mutex::new(HashMap::new())
        })
    }
}


#[tauri::command]
fn stop_all(state: State<AppData>) {
    state.sink.stop();
}

#[tauri::command]
async fn play_sound(sound: SoundInfo, app: tauri::AppHandle, state: State<'_, AppData>) -> std::result::Result<(), InvokeError> {
    // Stop currently playing sound
    state.sink.stop();

    let buffer = state.sounds_name.lock().unwrap().get(&sound.name).ok_or("No such sound")?.borrow_mut().sound.clone();
    state.sink.append(buffer);
    
    app.emit_all("now_playing", Some(sound))?;
    
    state.sink.sleep_until_end();

    app.emit_all("now_playing", Option::<SoundInfo>::None)?;

    Ok(())
}


#[tauri::command]
fn get_sounds(state: State<AppData>) -> std::result::Result<Vec<SoundInfo>, String> {
    Ok(
        state.sounds_name.lock().unwrap()
        .values().map(|sound| SoundInfo::from(sound.borrow()))
        .collect()
    )
}

#[tauri::command]
fn get_volume(state: State<AppData>) -> f32 {
    state.sink.volume()
}

#[tauri::command]
fn set_volume(volume: f32, state: State<AppData>, store: State<Mutex<Store<Wry>>>) -> std::result::Result<(), InvokeError> {
    let mut store = store.lock().unwrap();

    store.insert("volume".to_string(), json!(volume)).unwrap();
    store.save()?;

    state.sink.set_volume(volume);

    Ok(())
}

fn sounds_to_json(sounds: Vec<SoundInfo>) -> serde_json::Value {
    serde_json::to_value(sounds).unwrap()
}


#[tauri::command]
fn add_sound(sound: SoundInfo, state: State<AppData>, store: State<Mutex<Store<Wry>>>) -> std::result::Result<Vec<SoundInfo>, InvokeError> {
    let sound = Sound::from_path(sound.name, sound.path).map_err(|_| InvokeError::from("Couldn't create sound"))?;

    state.sounds_name.lock().unwrap().insert(sound.name.clone(), RefCell::new(sound));

    let json_sounds = sounds_to_json(state.sounds_name.lock().unwrap().values().map(|sound| SoundInfo::from(sound.borrow())).collect());
    
    {
        let mut store = store.lock().unwrap();
        store.insert("sounds".to_string(), json_sounds).unwrap();
        store.save().unwrap();
    }

    Ok(
        state.sounds_name.lock().unwrap()
        .values().map(|sound| SoundInfo::from(sound.borrow()))
        .collect()
    )
}

#[tauri::command]
fn delete_sound(sound: SoundInfo, app: tauri::AppHandle, state: State<AppData>, store: State<Mutex<Store<Wry>>>) -> std::result::Result<(), InvokeError> {
    state.sounds_name.lock().unwrap().remove(&sound.name);
    let json_sounds = sounds_to_json(state.sounds_name.lock().unwrap().values().map(|sound| SoundInfo::from(sound.borrow())).collect());

    let mut store = store.lock().unwrap();
    store.insert("sounds".to_string(), json_sounds).unwrap();
    store.save().unwrap();

    app.emit_all("force_refresh", ())?;

    Ok(())
}

fn main() {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();
    sink.set_volume(0.2);

    let app_data = AppData::try_default(sink).unwrap();


    tauri::Builder::default()
        .manage(app_data)
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let state: State<'_, AppData> = app.state();

            let mut store = StoreBuilder::new(app.handle(), "app_data.bin".parse()?).build();
            store.load().unwrap();

            if store.has("volume") {
                let volume = store.get("volume").unwrap().as_f64().unwrap() as f32;
                state.sink.set_volume(volume);
            }

            if !store.has("sounds") {
                println!("No sounds found, creating new store");
                store.insert("sounds".to_string(), Vec::<Value>::new().into())?;
                store.save().unwrap();
            } else {
                let sounds: Vec<SoundInfo> = serde_json::from_value(store.get("sounds").unwrap().to_owned()).unwrap();

                let state = app.state::<AppData>();

                {
                    let mut sound_map = state.sounds_name.lock().unwrap();

                    for sound in sounds {
                        let sound = Sound::from_soundinfo(sound).unwrap();
                        sound_map.insert(sound.name.clone(), RefCell::new(sound));
                    }
                }
            }

            app.handle().manage(Mutex::new(store));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            play_sound,
            get_sounds,
            get_volume,
            set_volume,
            add_sound,
            delete_sound,
            stop_all
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
