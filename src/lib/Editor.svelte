<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { EditorState } from "@codemirror/state";
  import { EditorView, basicSetup } from "codemirror";
  import { StreamLanguage } from "@codemirror/language";
  import { stex } from "@codemirror/legacy-modes/mode/stex";

  import { Compartment } from "@codemirror/state";
  import { languageServer } from "codemirror-languageserver";
  import { autocompletion, type CompletionContext, type CompletionResult } from "@codemirror/autocomplete";

  interface Props {
    content: string;
    filePath?: string;
    lspPort?: number | null;
    notes?: Array<{ path: string; title: string }>;
    allTags?: string[];
    onchange?: (content: string) => void;
    oncursorchange?: (line: number) => void;
  }
  
  let { content = $bindable(""), filePath, lspPort, notes = [], allTags = [], onchange, oncursorchange }: Props = $props();

  let editorContainer: HTMLElement;
  let view: EditorView | null = null;
  const lspCompartment = new Compartment();
  let currentFontSize = $state(14);

  function handleKeydown(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey) {
      if (e.key === '=' || e.key === '+') {
        currentFontSize = Math.min(currentFontSize + 1, 48);
        e.preventDefault();
      } else if (e.key === '-') {
        currentFontSize = Math.max(currentFontSize - 1, 8);
        e.preventDefault();
      } else if (e.key === '0') {
        currentFontSize = 14;
        e.preventDefault();
      }
    }
  }

  export function scrollToLine(lineNumber: number) {
    if (!view) return;
    try {
      const line = view.state.doc.line(lineNumber);
      view.dispatch({
        selection: { anchor: line.from },
        effects: EditorView.scrollIntoView(line.from, { y: "center" })
      });
    } catch (e) {
      console.warn("Line out of bounds", e);
    }
  }

  function customCompletions(context: CompletionContext): CompletionResult | null {
    // Note linking: autocomplete [[
    let word = context.matchBefore(/\[\[[^\]]*/);
    if (word) {
      if (word.from == word.to && !context.explicit) return null;
      return {
        from: word.from + 2, // Start replacing after [[
        options: notes.map(n => ({ 
          label: n.title,
          apply: n.path + "]]",
          type: "text",
          info: n.path
        })),
        validFor: /^[^\]]*$/
      };
    }

    // Tag linking: autocomplete #
    let tagWord = context.matchBefore(/#[a-zA-Z0-9_\-]*/);
    if (tagWord) {
      if (tagWord.from == tagWord.to && !context.explicit) return null;
      return {
        from: tagWord.from + 1, // Start replacing after #
        options: allTags.map(t => ({ 
          label: t,
          apply: t + " ",
          type: "keyword"
        })),
        validFor: /^[a-zA-Z0-9_\-]*$/
      };
    }

    return null;
  }

  onMount(() => {
    if (!editorContainer) return;

    const state = EditorState.create({
      doc: content,
      extensions: [
        basicSetup,
        EditorState.languageData.of(() => [{ autocomplete: customCompletions }]),
        StreamLanguage.define(stex),
        lspCompartment.of([]),
        EditorView.updateListener.of((v) => {
          if (v.docChanged) {
            const newDoc = v.state.doc.toString();
            if (newDoc !== content) {
                content = newDoc;
                if (onchange) {
                    onchange(newDoc);
                }
            }
          }
          if (v.selectionSet && oncursorchange) {
            const pos = v.state.selection.main.head;
            const line = v.state.doc.lineAt(pos).number;
            oncursorchange(line);
          }
        }),
        EditorView.theme({
          "&": {
            backgroundColor: "transparent",
            color: "var(--on-surface)",
            height: "100%",
            fontSize: "var(--editor-font-size, 14px)",
            fontFamily: "'JetBrains Mono', 'Fira Code', Consolas, monospace"
          },
          ".cm-content": {
            padding: "20px 0"
          },
          ".cm-gutters": {
            backgroundColor: "transparent",
            color: "var(--on-surface-variant)",
            borderRight: "1px solid var(--outline-variant)"
          },
          ".cm-activeLine, .cm-activeLineGutter": {
            backgroundColor: "transparent"
          },
          "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, & ::selection": {
            backgroundColor: "rgba(87, 27, 193, 0.4) !important",
            color: "inherit !important"
          }
        }, { dark: true })
      ]
    });

    view = new EditorView({
      state,
      parent: editorContainer
    });
  });

  onDestroy(() => {
    if (view) {
      view.destroy();
    }
  });

  // Watch for external content changes (e.g. when opening a new note)
  $effect(() => {
    if (view) {
      const normalizedContent = content.replace(/\r\n/g, '\n');
      if (normalizedContent !== view.state.doc.toString()) {
        view.dispatch({
          changes: { from: 0, to: view.state.doc.length, insert: normalizedContent }
        });
      }
    }
  });

  // Watch for LSP config changes to recreate the languageServer connection
  $effect(() => {
    if (view && lspPort && filePath) {
      try {
        const ls = languageServer({
          serverUri: `ws://127.0.0.1:${lspPort}`,
          rootUri: "file:///",
          documentUri: `file:///${filePath.replace(/\\/g, '/')}`,
          languageId: "latex"
        });
        view.dispatch({
          effects: lspCompartment.reconfigure(ls)
        });
      } catch (e) {
        console.warn("Failed to connect LSP", e);
      }
    }
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="editor-wrapper" bind:this={editorContainer} style="--editor-font-size: {currentFontSize}px"></div>

<style>
  .editor-wrapper {
    flex: 1;
    overflow: hidden;
    position: relative;
    background: transparent;
  }
</style>
