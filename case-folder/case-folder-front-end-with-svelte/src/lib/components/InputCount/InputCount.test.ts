// Pins the numeric stepper: increment/decrement adjust the spinbutton
// value, and the decrement button is disabled once the value reaches `min`.

import { describe, it, expect } from 'vitest';
import { render, fireEvent } from '@testing-library/svelte';
import InputCount from './InputCount.svelte';

describe('InputCount', () => {
    it('increments and decrements within bounds', async () => {
        const { getByRole } = render(InputCount, { props: { label: 'Copies', value: 2 } });
        const input = getByRole('spinbutton', { name: 'Copies' }) as HTMLInputElement;
        expect(input.value).toBe('2');

        await fireEvent.click(getByRole('button', { name: 'Increase Copies' }));
        expect(input.value).toBe('3');

        await fireEvent.click(getByRole('button', { name: 'Decrease Copies' }));
        expect(input.value).toBe('2');
    });

    it('disables decrement at the minimum', () => {
        const { getByRole } = render(InputCount, { props: { label: 'Copies', value: 1, min: 1 } });
        expect(getByRole('button', { name: 'Decrease Copies' })).toBeDisabled();
    });
});
