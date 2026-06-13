import '@testing-library/jest-dom/vitest';
import { afterEach } from 'vitest';
import { cleanup } from '@testing-library/svelte';

// Unmount rendered components between tests (we don't use vitest globals,
// so testing-library's auto-cleanup isn't registered for us).
afterEach(cleanup);
