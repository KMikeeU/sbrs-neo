<script lang="ts">
	import SoundBox from './components/SoundBox.svelte';
	import { invoke } from '@tauri-apps/api/tauri';
  import type { SoundInfo } from './data/SoundInfo';
  import { open } from '@tauri-apps/api/dialog';
    import { listen } from '@tauri-apps/api/event';

  let current_sound: SoundInfo | undefined = undefined;

  let sounds: Promise<[SoundInfo]>;
  
  function refreshSounds() {
    sounds = invoke("get_sounds");
  }

  listen("force_refresh", () => {
    refreshSounds();
  });

  listen("now_playing", (event) => {
    current_sound = event.payload as SoundInfo;
  });

  // The volume property is a number between 0 and 1 (inclusive)
  // It uses the getter "get_volume" and the setter "set_volume"
  
  let volume: number = 0;
  let volume_percent: number;

  invoke("get_volume").then(new_volume => {
    // Make sure new_volume is of type number
    if (typeof new_volume === "number")
      volume = new_volume;
  });

  $: {
    invoke("set_volume", { volume: volume })
  }

  const vol_to_percent = (vol: number) => { volume_percent = Math.round(vol * 100); };
  const percent_to_vol = (percent: number) => { volume = (percent / 100); };

  $: vol_to_percent(volume);
  $: percent_to_vol(volume_percent);

  async function addSound() {
    const soundPath = await open({
      multiple: false,
      filters: [{
        name: "Audio Files",
        extensions: ["mp3", "wav", "ogg", "flac"],
      }]
    });

    console.log(soundPath);

    if (soundPath == null) {
      return;
    }

    // If soundPath is an array, not a string, return
    if (Array.isArray(soundPath)) {
      return;
    }

    const soundName = soundPath.replace(/^.*[\\/]/, '').split(".")[0];

    if (soundName == null || soundName.length == 0) {
      return;
    }

    sounds = invoke("add_sound", {
      sound: {
        name: soundName, path: soundPath
      }
    });
  }

  async function stop_all() {
    await invoke("stop_all");
  }

  refreshSounds();
</script>

<main class="w-full gap-4 flex flex-col m-auto p-6 max-w-screen-md xl:max-w-screen-lg">
  <!-- Currently playing -->
  <div class="fixed bottom-0 left-0 p-4">
    <p>Currently playing: {current_sound != null ? current_sound.name : "None"}</p>
  </div>

  <!-- Controls -->
  <div class="flex gap-4 flex-col w-full">
    <div class="flex flex-col gap-2">
      <label for="">Controls</label>
      <div class="flex">
        <button class="max-w-fit" on:click={stop_all}>■ Stop Playback</button>
      </div>
    </div>
    <div class="flex flex-col gap-2">
      <label for="volume-input">Volume</label>
      <div class="flex gap-4 items-center">
        <input class="inline-block w-full p-0 accent-emerald-800" id="volume-input" type="range" min="0" max="1" step="0.01" bind:value={volume} />
        <input class="w-28 text-right" type="text" step="1" bind:value={volume_percent} />
      </div>
    </div>
  </div>

  <div class="grid w-100 gap-4 grid-cols-1 sm:grid-cols-2 xl:grid-cols-3">
    {#await sounds}
      <p>Loading...</p>
    {:then sounds}
      {#each sounds as sound}
        <div class="col">
          <SoundBox active="{current_sound?.name == sound.name}" {sound} />
        </div>
      {/each}
      <div class="col">
        <button class="w-full h-full min-h-24" on:click={addSound}>
          +
        </button>
      </div>
    {:catch error}
      <p>{error.message}</p>
    {/await}
  </div>


</main>

<style>

</style>