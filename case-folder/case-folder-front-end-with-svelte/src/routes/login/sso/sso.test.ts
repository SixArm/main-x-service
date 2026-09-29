import { describe, it, expect, vi, beforeEach } from 'vitest';
import { isHttpError, isRedirect } from '@sveltejs/kit';
import { oidcLoginUrl } from '$lib/server/auth';

const mockEnv: { PUBLIC_OIDC_SIGNIN_ENABLED?: string } = {};
vi.mock('$env/dynamic/public', () => ({ env: mockEnv }));

describe('oidcLoginUrl', () => {
    it('targets the auth service OIDC login with an encoded return_url', () => {
        const u = oidcLoginUrl('http://localhost:5251');
        expect(u).toMatch(/\/api\/auth\/oidc\/login\?return_url=/);
        expect(u.endsWith('return_url=http%3A%2F%2Flocalhost%3A5251')).toBe(
            true,
        );
    });
});

describe('GET /login/sso', () => {
    beforeEach(() => {
        delete mockEnv.PUBLIC_OIDC_SIGNIN_ENABLED;
    });

    it('404s when PUBLIC_OIDC_SIGNIN_ENABLED is unset', async () => {
        const { GET } = await import('./+server');
        const url = new URL('https://cf.example.test/login/sso');
        try {
            GET({ url } as Parameters<typeof GET>[0]);
            throw new Error('expected GET to throw');
        } catch (err) {
            expect(isHttpError(err)).toBe(true);
            if (isHttpError(err)) expect(err.status).toBe(404);
        }
    });

    it('303s to the OIDC login URL when enabled', async () => {
        mockEnv.PUBLIC_OIDC_SIGNIN_ENABLED = 'true';
        const { GET } = await import('./+server');
        const url = new URL('https://cf.example.test/login/sso');
        try {
            GET({ url } as Parameters<typeof GET>[0]);
            throw new Error('expected GET to throw');
        } catch (err) {
            expect(isRedirect(err)).toBe(true);
            if (isRedirect(err)) {
                expect(err.status).toBe(303);
                expect(err.location).toBe(
                    oidcLoginUrl('https://cf.example.test'),
                );
            }
        }
    });
});
