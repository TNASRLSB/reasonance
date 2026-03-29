/**
 * CodeMirror StateField for anchor-based search position tracking (W3.5).
 *
 * When search results are loaded for the active file we dispatch setAnchors
 * with the absolute positions of the matches. As the user types, CodeMirror
 * maps those positions through document changes so they stay valid. Navigating
 * to a result reads from the StateField rather than recomputing from the
 * original line number, which would be wrong after edits.
 */

import { StateField, StateEffect } from '@codemirror/state';

/**
 * Dispatch this effect to install a fresh set of anchor positions.
 * Each element is an absolute document offset (0-based).
 */
export const setAnchors = StateEffect.define<number[]>();

/**
 * A StateField that holds the current list of search anchor positions.
 *
 * On each transaction:
 *  1. If a setAnchors effect is present, replace the list.
 *  2. If the document changed, map each position through the change set so it
 *     follows inserted/deleted text.
 *  3. Filter out any positions that have been deleted (mapPos returns a
 *     negative value when assoc=-1 pushes the position out of a deletion).
 */
export const searchAnchorsField = StateField.define<number[]>({
  create() {
    return [];
  },

  update(positions, tr) {
    // Check for an explicit setAnchors effect first
    for (const effect of tr.effects) {
      if (effect.is(setAnchors)) {
        // Map the newly set positions through any changes in the same transaction
        const newPositions = effect.value;
        if (!tr.docChanged) return newPositions;
        return newPositions
          .map((pos) => tr.changes.mapPos(pos, 1))
          .filter((pos) => pos >= 0 && pos <= tr.newDoc.length);
      }
    }

    // No explicit update — just map existing positions through document changes
    if (!tr.docChanged) return positions;
    return positions
      .map((pos) => tr.changes.mapPos(pos, 1))
      .filter((pos) => pos >= 0 && pos <= tr.newDoc.length);
  },
});

/**
 * The CodeMirror extension to add to the editor so the field is active.
 */
export const searchAnchorsExtension = searchAnchorsField;
