<script lang="ts">
    import { api, ApiError } from '$lib/api/client';

    import Alert from '$lib/components/Alert/Alert.svelte';
    import Form from '$lib/components/Form/Form.svelte';
    import Field from '$lib/components/Field/Field.svelte';
    import Button from '$lib/components/Button/Button.svelte';

    let email = $state('');
    let emailError = $state('');
    let submitError = $state('');
    let sent = $state(false);
    let magicLink = $state<string | null>(null);

    async function handleSubmit() {
        emailError = '';
        submitError = '';
        sent = false;
        magicLink = null;

        if (!email.trim()) {
            emailError = 'Enter your email address.';
            return;
        }

        try {
            const res = await api.auth.requestLink(email.trim());
            sent = res.sent;
            magicLink = res.magicLink;
        } catch (e) {
            if (e instanceof ApiError && e.status === 422) {
                const body = e.body as { errors?: Record<string, string> } | null;
                emailError = body?.errors?.email ?? e.message;
            } else {
                submitError = (e as Error).message;
            }
        }
    }
</script>

<h2>Sign in</h2>
<p>
    Enter your work email address. If it is recognised, we'll send you a
    one-time sign-in link. No password required.
</p>

{#if submitError}
    <Alert type="error" heading="Could not send sign-in link">{submitError}</Alert>
{/if}

{#if sent}
    <Alert type="success" heading="Check your email">
        If <strong>{email}</strong> matches a known account, a sign-in link is on
        its way. The link expires in 10 minutes.
    </Alert>
    {#if magicLink}
        <!-- Dev convenience: the API exposes the link directly so you can
             click it without an email server. Never shown in production. -->
        <p class="dev-magic-link">
            Development shortcut:
            <a href={magicLink} data-testid="magic-link">open your sign-in link</a>
        </p>
    {/if}
{:else}
    <Form label="Sign in" onsubmit={handleSubmit}>
        <Field label="Email address" required error={emailError}>
            <input type="email" bind:value={email} required autocomplete="email" />
        </Field>
        <div class="actions">
            <Button type="submit">Email me a sign-in link</Button>
        </div>
    </Form>
{/if}
