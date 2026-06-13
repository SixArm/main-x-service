import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import Icon from './Icon.svelte';

describe('Icon', () => {
    it('renders an inline svg for a known icon', () => {
        const { container } = render(Icon, { props: { name: 'folder' } });
        expect(container.querySelector('svg')).toBeTruthy();
    });

    it('exposes a label as an accessible image', () => {
        const { getByRole } = render(Icon, { props: { name: 'printer', label: 'Print' } });
        expect(getByRole('img', { name: 'Print' })).toBeInTheDocument();
    });

    it('renders nothing for an unknown icon name', () => {
        const { container } = render(Icon, { props: { name: 'does-not-exist' } });
        expect(container.querySelector('svg')).toBeNull();
    });
});
