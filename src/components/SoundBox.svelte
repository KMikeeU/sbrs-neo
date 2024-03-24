<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
    import type { SoundInfo } from "../data/SoundInfo";

    export let sound: SoundInfo;
    export let active: boolean;


    async function play() {
        await invoke("play_sound", { sound });
    }

    let delete_confirm = false;

    function on_delete() {
        if (!delete_confirm) {
            delete_confirm = true;
            return;
        }

        invoke("delete_sound", { sound });
    }
</script>

<div on:mouseleave={() => setTimeout(() => delete_confirm = false, 100)} class="group ring-emerald-800 {active ? 'ring-2' : ''}" role="navigation">
    <div class="transition-all group-hover:opacity-100 opacity-0 border-[1px] border-zinc-700 text-sm flex absolute m-2 rounded-md overflow-hidden">
        <button class="border-r-[1px] border-zinc-700 p-1 hover:bg-zinc-800" on:click={on_delete}>{delete_confirm ? '❓' : '🗑️'}</button>
        <button class="p-1 hover:bg-zinc-800">✒️</button>
    </div>

    <button on:click={play}>
        <div class="content">
            <h2>{sound.name}</h2>
            <p>{sound.path}</p>
        </div>
    </button>
</div>

<style>
    button {
        width: 100%;
        height: 100%;
    }

    .content {
        margin: 1.5rem 2rem;
    }

    h2 {
        font-size: 1.2rem;
        margin: 0;
    }

    p {
        font-size: 0.8rem;
        color: gray;
        margin: 0;

        direction: rtl;
        overflow: hidden;
        white-space: nowrap;
        text-overflow: ellipsis;
    }
</style>