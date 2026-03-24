import { literal } from 'lit/static-html.js';
import type { BlockSpec } from '@blocksuite/block-std';
import { AgentNoteBlockSchema } from './agent-note-schema';
import { AgentNoteBlockService } from './agent-note-service';

export const AgentNoteBlockSpec: BlockSpec = {
  schema: AgentNoteBlockSchema,
  service: AgentNoteBlockService,
  view: {
    component: literal`sparkdown-agent-note`,
  },
};
