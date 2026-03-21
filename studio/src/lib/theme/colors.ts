export const ENTITY_COLORS: Record<string, string> = {
  'schema:Person': '#F59E0B',
  'schema:Place': '#14B8A6',
  'schema:Event': '#8B5CF6',
  'schema:Organization': '#6366F1',
  'schema:Article': '#22C55E',
  'schema:CreativeWork': '#22C55E',
  'sd:Review': '#F43F5E',
  'sd:Abstract': '#F43F5E',
  'sd:Argument': '#F43F5E',
  'foaf:Person': '#F59E0B',
  'foaf:Organization': '#6366F1',
};

const DEFAULT_COLOR = '#6B7280';

export function entityColor(typePrefix: string): string {
  return ENTITY_COLORS[typePrefix] ?? DEFAULT_COLOR;
}
