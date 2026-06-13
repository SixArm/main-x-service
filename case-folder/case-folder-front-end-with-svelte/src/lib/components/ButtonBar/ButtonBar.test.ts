import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import ButtonBar from './ButtonBar.svelte';

describe('ButtonBar', () => {
    it('renders the default actions and emits the key on click', async () => {
        const onselect = vi.fn();
        const { getByRole } = render(ButtonBar, { props: { onselect } });

        expect(getByRole('button', { name: 'Patient' })).toBeInTheDocument();
        expect(getByRole('button', { name: 'Case Notes' })).toBeInTheDocument();

        await fireEvent.click(getByRole('button', { name: 'Case Notes' }));
        expect(onselect).toHaveBeenCalledWith('case-notes');
    });
});
