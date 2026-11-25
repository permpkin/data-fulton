import type { Catalog, EntitySpec, ChildSpec } from '@shared/types';

/**
 * A connection provider: everything the UI needs to drive its
 * connection form and field-selection step for one platform.
 */
export interface ProviderField {
  key: string;
  label: string;
  placeholder: string;
  password?: boolean;
  /** Optional "how to connect" help rendered under this field. */
  instructions?: { title: string; steps: string[] };
}

export interface Provider {
  id: string;
  name: string;
  /** Connection-form fields (rendered in order, after the name field). */
  fields: ProviderField[];
  /** Default host pre-filled on new connections. */
  defaultHost: string;
  /** Endpoint hit to validate a connection/key. */
  validatePath: string;
  /** Extraction field catalog for this provider. */
  catalog: () => Promise<Catalog>;
  /** Entity specs embedded into a session config for the extractor. */
  entitySpecs: EntitySpec[];
  /** Child (per-parent) specs embedded into a session config. */
  childSpecs: ChildSpec[];
}
