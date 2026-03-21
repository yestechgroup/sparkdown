import { ViewPlugin } from '@codemirror/view';
import type { ViewUpdate, EditorView } from '@codemirror/view';

export type EditorMode = 'deep-writing' | 'light-writing' | 'review' | 'full-reading';

/**
 * Mode transition plugin that monitors typing and scrolling activity
 * to determine the current mode:
 *
 * - deep-writing: user is actively typing
 * - light-writing: user paused typing for 2s (entity underlines fade in)
 * - review: user scrolling without typing for 5s (knowledge panel slides in)
 * - full-reading: toggled manually via Cmd+Shift+R
 *
 * Typing at any time returns to deep-writing.
 */
export function createModeTransitionPlugin(
  onModeChange: (mode: EditorMode) => void,
) {
  return ViewPlugin.fromClass(
    class {
      private typingTimer: ReturnType<typeof setTimeout> | null = null;
      private scrollTimer: ReturnType<typeof setTimeout> | null = null;
      private currentMode: EditorMode = 'deep-writing';
      private lastTypedAt = 0;

      constructor(_view: EditorView) {}

      update(update: ViewUpdate) {
        if (update.docChanged) {
          this.lastTypedAt = Date.now();

          // Return to deep-writing on any typing
          if (this.currentMode !== 'deep-writing') {
            this.setMode('deep-writing', onModeChange);
          }

          // Reset typing timer for light-writing transition
          if (this.typingTimer) clearTimeout(this.typingTimer);
          this.typingTimer = setTimeout(() => {
            if (this.currentMode === 'deep-writing') {
              this.setMode('light-writing', onModeChange);
            }
          }, 2000);

          // Reset scroll timer
          if (this.scrollTimer) clearTimeout(this.scrollTimer);
        }

        if (update.viewportChanged && !update.docChanged) {
          // Scrolling without typing
          if (this.scrollTimer) clearTimeout(this.scrollTimer);
          const timeSinceTyped = Date.now() - this.lastTypedAt;

          if (timeSinceTyped > 2000) {
            this.scrollTimer = setTimeout(() => {
              if (
                this.currentMode === 'light-writing' ||
                this.currentMode === 'deep-writing'
              ) {
                this.setMode('review', onModeChange);
              }
            }, 5000);
          }
        }
      }

      private setMode(
        mode: EditorMode,
        cb: (mode: EditorMode) => void,
      ) {
        if (this.currentMode !== mode) {
          this.currentMode = mode;
          cb(mode);
        }
      }

      destroy() {
        if (this.typingTimer) clearTimeout(this.typingTimer);
        if (this.scrollTimer) clearTimeout(this.scrollTimer);
      }
    },
  );
}
