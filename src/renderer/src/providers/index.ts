import { PRIME_ECO } from './primeeco';
import { BUILDPASS } from './buildpass';
import { PROCORE } from './procore';
import { SIMPRO } from './simpro';
import { AROFLOW } from './aroflow';
import { MONDAY } from './monday';
import type { Provider } from './types';

export const PROVIDERS: Provider[] = [PRIME_ECO, BUILDPASS, PROCORE, SIMPRO, AROFLOW, MONDAY];

export const getProvider = (id: string): Provider =>
  PROVIDERS.find((p) => p.id === id) ?? PRIME_ECO;
