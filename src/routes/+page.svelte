<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import Editor from '$lib/Editor.svelte';
  import GraphView from '$lib/GraphView.svelte';

  const appWindow = getCurrentWindow();

  type NoteSummary = {
    path: string;
    title: string;
  };

  let vaultPath: string = $state("C:/Users/varia/Documents/Development/Latte/default_vault");

  let notes: NoteSummary[] = $state([]);
  let currentNote: NoteSummary | null = $state(null);
  let content: string = $state("");

  let compileLog: string = $state("");
  let editorRef: any = $state();
  let compileTimeout: any;
  let sumatraTimeout: any;
  let currentCursorLine: number = $state(1);

  let backlinks: string[] = $state([]);
  let noteTags: string[] = $state([]);
  let allTags: string[] = $state([]);
  
  let showGraph = $state(false);
  let graphData: any = $state({ nodes: [], links: [] });
  let searchQuery = $state("");
  let searchResults: string[] = $state([]);

  let lspPort: number | null = $state(null);

  // Bottom panel state
  let showLog = $state(true);

  async function fetchContext(path: string) {
    try {
      backlinks = await invoke("get_backlinks", { notePath: path });
      noteTags = await invoke("get_tags", { notePath: path });
    } catch (e) {
      console.error("Failed to fetch context", e);
    }
  }

  async function loadNotes() {
    try {
      notes = await invoke("list_notes", { vaultPath, dbState: {} });
      lspPort = await invoke("get_lsp_port");
      allTags = await invoke("get_all_tags", { dbState: {} });
    } catch (e) {
      console.error("Failed to load notes or lsp port", e);
    }
  }

  async function openNote(note: NoteSummary) {
    try {
      const text: string = await invoke("read_note", { vaultPath, notePath: note.path });
      currentNote = note;
      content = text;
      fetchContext(note.path);
      compileNote();
    } catch (e) {
      console.error("Failed to read note", e);
    }
  }

  async function saveNote() {
    if (!currentNote) return;
    try {
      await invoke("save_note", { vaultPath, notePath: currentNote.path, content });
      fetchContext(currentNote.path);
    } catch (e) {
      console.error("Failed to save note", e);
    }
  }

  async function selectVault() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select Vault Directory"
      });
      if (selected && typeof selected === "string") {
        vaultPath = selected;
        currentNote = null;
        content = "";
        compileLog = "";
        await loadNotes();
      }
    } catch (e) {
      console.error("Failed to select vault", e);
    }
  }

  async function createNote() {
    let baseName = "Untitled";
    let fileName = `${baseName}.tex`;
    let counter = 1;
    
    while (notes.some(n => n.path === fileName)) {
      fileName = `${baseName} ${counter}.tex`;
      counter++;
    }
    
    try {
      await invoke("create_file", { vaultPath, fileName });
      await loadNotes();
      const n = notes.find(x => x.path === fileName);
      if (n) openNote(n);
    } catch (e) {
      console.error("Failed to create file", e);
    }
  }

  async function createFolder() {
    const folderName = prompt("Enter folder name:", "New Folder");
    if (!folderName) return;
    try {
      await invoke("create_folder", { vaultPath, folderName });
    } catch (e) {
      console.error("Failed to create folder", e);
      alert("Failed to create folder: " + e);
    }
  }

  async function renameNote(note: NoteSummary, newName: string) {
    if (!newName.trim() || newName === note.path) return;
    const oldName = note.path;
    
    if (notes.some(n => n.path === newName)) {
      alert("A file with this name already exists.");
      return;
    }
    
    try {
      await invoke("rename_file", { vaultPath, oldName, newName });
      await loadNotes();
      const n = notes.find(x => x.path === newName);
      if (n) currentNote = n;
    } catch (e) {
      console.error("Failed to rename file", e);
    }
  }

  async function deleteNote(note: NoteSummary) {
    if (!confirm(`Are you sure you want to delete ${note.title}? This cannot be undone.`)) return;
    
    try {
      await invoke("delete_file", { vaultPath, fileName: note.path });
      currentNote = null;
      content = "";
      compileLog = "";
      await loadNotes();
    } catch (e) {
      console.error("Failed to delete file", e);
    }
  }

  async function openGraph() {
    try {
      graphData = await invoke("get_graph_data", { dbState: {} });
      showGraph = true;
    } catch (e) {
      console.error("Failed to load graph data", e);
    }
  }

  async function handleSearch() {
    if (!searchQuery.trim()) {
      searchResults = [];
      return;
    }
    try {
      searchResults = await invoke("search_notes", { query: searchQuery });
    } catch (e) {
      console.error("Search failed", e);
    }
  }

  function handleContentChange() {
    clearTimeout(compileTimeout);
    compileTimeout = setTimeout(() => {
      saveNote();
      compileNote();
    }, 800);
  }

  async function compileNote() {
    if (!content || !currentNote) return;
    if (!currentNote.path.endsWith('.tex')) {
      compileLog = "Compilation skipped: Not a TeX file.";
      return;
    }
    try {
      const result: any = await invoke("compile_note", { 
        vaultPath, 
        notePath: currentNote.path,
        content: content
      });
      compileLog = result.log;
      if (result.ok) {
        syncSumatra(currentCursorLine);
      }
    } catch (e) {
      console.error("Compilation error", e);
    }
  }

  async function syncSumatra(line: number) {
    if (!currentNote || !currentNote.path.endsWith('.tex')) return;
    try {
      await invoke("open_sumatra", { 
        pdfPath: `${vaultPath}/${currentNote.path.replace('.tex', '.pdf')}`,
        texPath: `${vaultPath}/${currentNote.path}`,
        line: line
      });
    } catch (e) {
      console.error("Sumatra error", e);
    }
  }

  function handleCursorChange(line: number) {
    currentCursorLine = line;
  }

  onMount(() => {
    loadNotes();
  });
</script>

<main class="window-wrapper">
  <div class="titlebar">
    <div class="titlebar-title" data-tauri-drag-region>
      <svg data-tauri-drag-region xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#007aff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="margin-right: 6px; pointer-events: none;"><path d="M17 8h1a4 4 0 1 1 0 8h-1"></path><path d="M3 8h14v9a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4Z"></path><line x1="6" y1="2" x2="6" y2="4"></line><line x1="10" y1="2" x2="10" y2="4"></line><line x1="14" y1="2" x2="14" y2="4"></line></svg>
      llate
    </div>
    <div class="titlebar-drag-area" data-tauri-drag-region style="flex: 1; height: 100%;"></div>
    <div class="titlebar-controls">
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="titlebar-button" id="titlebar-minimize" onclick={() => appWindow.minimize()}>
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="5" y1="12" x2="19" y2="12"></line></svg>
      </div>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="titlebar-button" id="titlebar-maximize" onclick={() => appWindow.toggleMaximize()}>
        <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect></svg>
      </div>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="titlebar-button" id="titlebar-close" onclick={() => appWindow.close()}>
        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
      </div>
    </div>
  </div>

<div class="app-container">
  <aside class="sidebar">
    <div class="sidebar-header">
      <div class="vault-info">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="vault-icon"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"></path></svg>
        <h2>{vaultPath.split(/[/\\]/).pop()}</h2>
      </div>
      <div class="header-actions">
        <button class="icon-btn" title="New Note" onclick={createNote}>
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"></path><polyline points="14 2 14 8 20 8"></polyline><line x1="12" y1="18" x2="12" y2="12"></line><line x1="9" y1="15" x2="15" y2="15"></line></svg>
        </button>
        <button class="icon-btn" title="New Folder" onclick={createFolder}>
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path><line x1="12" y1="11" x2="12" y2="17"></line><line x1="9" y1="14" x2="15" y2="14"></line></svg>
        </button>
        <button class="icon-btn" title="Open Vault" onclick={selectVault}>
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.93a2 2 0 0 1 1.66.9l.82 1.2a2 2 0 0 0 1.66.9H18a2 2 0 0 1 2 2v2"></path></svg>
        </button>
        <button class="icon-btn" title="Graph View" onclick={openGraph}>
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="18" cy="5" r="3"></circle><circle cx="6" cy="12" r="3"></circle><circle cx="18" cy="19" r="3"></circle><line x1="8.59" y1="13.51" x2="15.42" y2="17.49"></line><line x1="15.41" y1="6.51" x2="8.59" y2="10.49"></line></svg>
        </button>
      </div>
    </div>
    
    <div class="search-box">
      <input 
        type="text" 
        placeholder="Search notes..." 
        bind:value={searchQuery} 
        oninput={handleSearch}
      />
    </div>

    <div class="notes-container">
      {#if searchQuery.trim().length > 0}
        <div class="section-label">Search Results</div>
        <ul class="note-list search-results-list">
          {#each searchResults as path}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_interactive_supports_focus -->
            <li 
              class:active={currentNote?.path === path}
              onclick={() => {
                let n = notes.find(n => n.path === path);
                if (n) openNote(n);
              }}
              role="button"
            >
              {path}
            </li>
          {:else}
            <li class="empty-text" style="padding: 10px;">No matches found</li>
          {/each}
        </ul>
        <div class="section-label">All Notes</div>
      {/if}

      <ul class="note-list">
        {#each notes as note}
          <!-- svelte-ignore a11y_interactive_supports_focus -->
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <li 
            class:active={currentNote?.path === note.path}
            onclick={() => openNote(note)}
            onkeydown={(e) => e.key === 'Enter' && openNote(note)}
            role="button"
          >
            {note.title}
          </li>
        {/each}
      </ul>
    </div>
    
    {#if currentNote}
    <div class="context-panel">
      <h4>Tags</h4>
      <div class="tags-list">
        {#each noteTags as tag}
          <span class="tag">#{tag}</span>
        {:else}
          <span class="empty-text">No tags</span>
        {/each}
      </div>
      
      <h4>Backlinks</h4>
      <ul class="backlinks-list">
        {#each backlinks as link}
          <li>
            <!-- svelte-ignore a11y_invalid_attribute -->
            <a href="#" onclick={(e) => {
               e.preventDefault();
               let n = notes.find(n => n.path === link);
               if (n) openNote(n);
            }}>{link}</a>
          </li>
        {:else}
          <li class="empty-text">No backlinks</li>
        {/each}
      </ul>
    </div>
    {/if}
  </aside>

  {#if showGraph}
    <GraphView 
      data={graphData} 
      onClose={() => showGraph = false} 
      onNodeClick={(path) => {
        let n = notes.find(n => n.path === path);
        if (n) {
          openNote(n);
          showGraph = false;
        }
      }} 
    />
  {/if}

  <section class="main-content">
    {#if currentNote}
      <div class="editor-header">
        <div class="note-title-area">
          <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="file-icon"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"></path><polyline points="14 2 14 8 20 8"></polyline></svg>
          <input 
            type="text" 
            class="title-input" 
            value={currentNote.path} 
            onblur={(e) => renameNote(currentNote, e.currentTarget.value)}
            onkeydown={(e) => e.key === 'Enter' && e.currentTarget.blur()}
          />
        </div>
        <div class="editor-actions">
          {#if currentNote.path.endsWith('.tex')}
          <button class="action-btn sumatra-btn" onclick={() => syncSumatra(currentCursorLine)}>
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z"></path><path d="m9 12 2 2 4-4"></path></svg>
            Sync SumatraPDF
          </button>
          {/if}
          <button class="action-btn delete-btn" onclick={() => deleteNote(currentNote)} title="Delete Note">
            <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18"></path><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"></path><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"></path></svg>
          </button>
        </div>
      </div>
      
      <div class="editor-container">
        <Editor 
          bind:this={editorRef}
          bind:content 
          filePath={vaultPath + "/" + currentNote.path}
          lspPort={lspPort}
          notes={notes}
          allTags={allTags}
          onchange={handleContentChange} 
          oncursorchange={handleCursorChange} 
        />
      </div>

      <!-- Bottom Log Panel -->
      <div class="bottom-panel" class:open={showLog}>
        <div class="bottom-panel-header" onclick={() => showLog = !showLog} role="button" tabindex="0" onkeydown={(e) => e.key === 'Enter' && (showLog = !showLog)}>
          <span>Compilation Log</span>
          <svg class="chevron" class:rotated={!showLog} xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="6 9 12 15 18 9"></polyline></svg>
        </div>
        {#if showLog}
          <div class="compile-log">
            {#if compileLog}
              {compileLog}
            {:else}
              <span class="empty-log">No compilation errors. Waiting for changes...</span>
            {/if}
          </div>
        {/if}
      </div>

    {:else}
      <div class="empty-state">
        <div class="empty-state-content">
          <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M21.21 15.89A10 10 0 1 1 8 2.83"></path><path d="M22 12A10 10 0 0 0 12 2v10z"></path></svg>
          <p>Select or create a note to begin editing.</p>
        </div>
      </div>
    {/if}
  </section>
</div>
</main>

<style>
  :global(:root) {
    /* Colors */
    --surface: #12131a;
    --surface-dim: #12131a;
    --surface-bright: #383941;
    --surface-container-lowest: #0d0e15;
    --surface-container-low: #1a1b22;
    --surface-container: #1e1f26;
    --surface-container-high: #292931;
    --surface-container-highest: #33343c;
    --on-surface: #e3e1ec;
    --on-surface-variant: #c7c4d7;
    --outline: #908fa0;
    --outline-variant: #464554;
    --primary: #c0c1ff;
    --on-primary: #1000a9;
    --primary-container: #8083ff;
    --on-primary-container: #0d0096;
    --error: #ffb4ab;
    --on-error: #690005;
    --background: #12131a;
    --on-background: #e3e1ec;
    --surface-variant: #33343c;

    /* Typography */
    --font-ui: 'Inter', -apple-system, sans-serif;
    --font-code: 'JetBrains Mono', monospace;

    /* Radii */
    --radius-sm: 0.125rem;
    --radius-default: 0.25rem;
    --radius-md: 0.375rem;
    --radius-lg: 0.5rem;
    --radius-xl: 0.75rem;
  }
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: var(--font-ui);
    background: var(--background);
    background-image: radial-gradient(circle at 50% 0%, rgba(192, 193, 255, 0.15), var(--background) 70%);
    color: var(--on-background);
    height: 100vh;
    overflow: hidden;
  }
  
  /* Scrollbar customization */
  :global(*::-webkit-scrollbar) {
    width: 6px;
    height: 6px;
  }
  :global(::-webkit-scrollbar-track) {
    background: transparent;
  }
  :global(::-webkit-scrollbar-thumb) {
    background: rgba(255, 255, 255, 0.1);
    border-radius: 6px;
  }
  :global(::-webkit-scrollbar-thumb:hover) {
    background: rgba(255, 255, 255, 0.2);
  }

  .window-wrapper {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
    background: transparent;
  }

  .titlebar {
    height: 36px;
    background: transparent;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--outline-variant);
    user-select: none;
    flex-shrink: 0;
    z-index: 50;
  }
  
  .titlebar-title {
    font-size: 14px;
    font-weight: 500;
    color: var(--on-surface-variant);
    margin-left: 16px;
    letter-spacing: -0.01em;
    display: flex;
    align-items: center;
  }
  
  .titlebar-controls {
    display: flex;
    height: 100%;
  }
  
  .titlebar-button {
    width: 46px;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
    cursor: pointer;
    color: #a1a1aa;
    pointer-events: auto;
    transition: background-color 0.2s, color 0.2s;
  }
  
  .titlebar-button:hover {
    background: rgba(255, 255, 255, 0.1);
    color: #ffffff;
  }
  
  #titlebar-close:hover {
    background: rgba(239, 68, 68, 0.8);
    color: #ffffff;
  }

  .app-container {
    display: flex;
    flex: 1;
    min-height: 0;
    background: transparent;
  }

  .sidebar {
    width: 280px;
    background: rgba(30, 31, 38, 0.4); /* surface-container with opacity */
    backdrop-filter: blur(24px);
    -webkit-backdrop-filter: blur(24px);
    border-right: 1px solid var(--outline-variant);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    flex-shrink: 0;
    z-index: 10;
  }

  .sidebar-header {
    padding: 20px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .vault-info {
    display: flex;
    align-items: center;
    gap: 12px;
    overflow: hidden;
  }

  .vault-icon {
    color: var(--primary);
    flex-shrink: 0;
    filter: drop-shadow(0 0 8px rgba(192, 193, 255, 0.4));
  }

  .sidebar h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--on-surface);
    letter-spacing: -0.01em;
  }

  .header-actions {
    display: flex;
    gap: 6px;
  }

  .icon-btn {
    background: transparent;
    border: 1px solid transparent;
    color: var(--on-surface-variant);
    cursor: pointer;
    padding: 6px;
    border-radius: var(--radius-md);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .icon-btn:hover {
    background: rgba(255, 255, 255, 0.05);
    color: var(--on-surface);
    transform: translateY(-1px) scale(1.05);
  }

  .search-box {
    padding: 0 20px 20px;
  }

  .search-box input {
    width: 100%;
    padding: 8px 12px;
    background: var(--surface-container-high);
    border: 1px solid var(--outline-variant);
    color: var(--on-surface);
    border-radius: var(--radius-md);
    box-sizing: border-box;
    font-size: 14px;
    font-family: inherit;
    transition: all 0.2s;
  }

  .search-box input:focus {
    outline: none;
    border-color: var(--primary);
    box-shadow: 0 0 0 3px rgba(192, 193, 255, 0.2);
    background: var(--surface-container-highest);
  }

  .search-box input::placeholder {
    color: var(--on-surface-variant);
  }

  .notes-container {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    padding: 0 12px;
  }

  .section-label {
    padding: 12px 8px 6px;
    font-size: 12px;
    font-weight: 500;
    color: var(--on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }

  .note-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .note-list li {
    padding: 8px 12px;
    cursor: pointer;
    user-select: none;
    border-radius: var(--radius-md);
    font-size: 14px;
    margin-bottom: 4px;
    color: var(--on-surface-variant);
    transition: all 0.2s ease;
    display: flex;
    align-items: center;
    border: 1px solid transparent;
  }

  .note-list li:hover {
    background: var(--surface-container-high);
    color: var(--on-surface);
    transform: translateX(2px);
  }

  .note-list li.active {
    background: var(--primary-container);
    border: 1px solid var(--primary);
    color: var(--on-primary-container);
    font-weight: 500;
    box-shadow: 0 4px 12px rgba(128, 131, 255, 0.2);
  }

  .context-panel {
    border-top: 1px solid var(--outline-variant);
    padding: 16px 20px;
    background: var(--surface-container-lowest);
    max-height: 250px;
    overflow-y: auto;
  }

  .context-panel h4 {
    margin: 0 0 12px 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }

  .tags-list {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 20px;
  }

  .tag {
    background: var(--surface-container-high);
    color: var(--primary);
    padding: 4px 10px;
    border-radius: var(--radius-md);
    font-size: 12px;
    border: 1px solid var(--outline-variant);
    transition: all 0.2s;
  }
  
  .tag:hover {
    background: var(--surface-container-highest);
    border-color: var(--outline);
    transform: translateY(-1px);
  }

  .backlinks-list {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .backlinks-list li {
    margin-bottom: 8px;
  }

  .backlinks-list a {
    color: var(--primary);
    text-decoration: none;
    font-size: 14px;
    transition: all 0.2s;
  }

  .backlinks-list a:hover {
    color: var(--primary-container);
    text-decoration: underline;
  }

  .empty-text {
    color: var(--on-surface-variant);
    font-size: 14px;
    font-style: italic;
    opacity: 0.7;
  }

  /* Main Content Area */
  .main-content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    position: relative;
    background: transparent;
  }

  .editor-header {
    height: 50px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 20px;
    background: transparent;
    z-index: 5;
    border-bottom: 1px solid var(--outline-variant);
  }

  .note-title-area {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .file-icon {
    color: var(--on-surface-variant);
    width: 16px;
    height: 16px;
  }

  .title-input {
    background: transparent;
    border: 1px solid transparent;
    color: var(--on-surface);
    font-size: 16px;
    font-weight: 500;
    font-family: inherit;
    padding: 4px 8px;
    border-radius: var(--radius-md);
    outline: none;
    transition: all 0.2s;
    width: 300px;
  }
  .title-input:focus, .title-input:hover {
    background: var(--surface-container);
  }

  .editor-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    background: transparent;
    color: var(--on-surface-variant);
    border: 1px solid var(--outline-variant);
    padding: 6px 12px;
    border-radius: var(--radius-md);
    font-size: 12px;
    cursor: pointer;
    font-family: inherit;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .action-btn:hover {
    background: var(--surface-container-high);
    color: var(--on-surface);
    transform: translateY(-1px);
    box-shadow: 0 4px 12px rgba(0,0,0,0.2);
  }

  .action-btn.sumatra-btn:hover {
    border-color: var(--primary);
    color: var(--primary);
  }

  .action-btn.delete-btn:hover {
    background: var(--error-container);
    border-color: var(--error);
    color: var(--on-error-container);
  }

  .editor-container {
    flex: 1;
    min-height: 0;
    position: relative;
    display: flex;
    flex-direction: column;
  }

  /* Bottom Panel */
  .bottom-panel {
    background: var(--surface-container);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-lg);
    margin: 16px 20px;
    padding: 12px 16px;
    display: flex;
    flex-direction: column;
    max-height: 40vh;
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    z-index: 20;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.4);
    transform: translateY(calc(100% - 36px));
    transition: transform 0.4s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .bottom-panel.open {
    transform: translateY(0);
  }

  .bottom-panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--outline-variant);
    margin-bottom: 12px;
    background: transparent;
    cursor: pointer;
    font-size: 12px;
    font-weight: 600;
    color: var(--on-surface-variant);
    text-transform: uppercase;
    letter-spacing: 0.1em;
    user-select: none;
    transition: color 0.2s;
  }

  .bottom-panel-header:hover {
    color: var(--on-surface);
  }

  .chevron {
    transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .chevron.rotated {
    transform: rotate(180deg);
  }

  .compile-log {
    margin: 0;
    padding: 12px 16px;
    font-size: 13px;
    color: var(--on-surface-variant);
    background: var(--surface-container-lowest);
    border: 1px solid var(--outline-variant);
    border-radius: var(--radius-md);
    white-space: pre-wrap;
    font-family: var(--font-code);
    line-height: 1.5;
    overflow-y: auto;
    flex: 1;
  }

  .empty-log {
    color: var(--on-surface-variant);
    font-style: italic;
    opacity: 0.7;
  }

  .empty-state {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .empty-state-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 20px;
    color: #333;
  }

  .empty-state-content svg {
    color: #222;
    filter: drop-shadow(0 4px 12px rgba(0,0,0,0.5));
  }

  .empty-state-content p {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 500;
    color: #555;
    letter-spacing: -0.01em;
  }
</style>
