import { defineBlockSchema } from '@blocksuite/store';

export const AgentNoteBlockSchema = defineBlockSchema({
  flavour: 'sparkdown:agent-note',
  props: () => ({
    agentId: '' as string,
    agentName: '' as string,
    noteType: 'entity' as 'entity' | 'summary' | 'question',
    content: '' as string,
    confidence: 0 as number,
    accepted: false as boolean,
  }),
  metadata: {
    version: 1,
    role: 'content',
    parent: ['affine:note'],
  },
});
