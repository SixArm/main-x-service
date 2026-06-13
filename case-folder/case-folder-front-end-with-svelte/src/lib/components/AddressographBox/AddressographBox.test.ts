import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/svelte';
import AddressographBox from './AddressographBox.svelte';

describe('AddressographBox', () => {
    it('renders the patient identifiers in a labelled region', () => {
        const { getByRole, getByText } = render(AddressographBox, {
            props: { name: 'Alice Johnson', nhsNumber: '943 476 5919', dateOfBirth: '1991-04-12' }
        });
        expect(getByRole('region', { name: 'Patient addressograph' })).toBeInTheDocument();
        expect(getByText('Alice Johnson')).toBeInTheDocument();
        expect(getByText('1991-04-12')).toBeInTheDocument();
    });
});
