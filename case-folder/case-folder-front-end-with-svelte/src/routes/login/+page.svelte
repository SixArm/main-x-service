<script lang="ts">
    // Login (`/login`) — passwordless magic-link sign-in.
    //
    // The user enters their email; the API mails a one-time link (and, in
    // dev/test, returns the link inline so it can be clicked without an
    // email server). The response is deliberately ambiguous about whether
    // the email is known, so the success copy never confirms account
    // existence. A 422 surfaces as a field error; anything else as a banner.
    //
    // State:
    //   email      — the entered address.
    //   emailError — per-field validation / 422 message.
    //   submitError— non-validation request failure.
    //   sent       — true once the request resolved, to swap the form for
    //                the confirmation message.
    //   magicLink  — dev-only direct link, when the API provides one.

    import { api, ApiError } from '$lib/api/client';

    import Alert from '$lib/components/Alert/Alert.svelte';
    import Form from '$lib/components/Form/Form.svelte';
    import Field from '$lib/components/Field/Field.svelte';
    import Button from '$lib/components/Button/Button.svelte';
    import { t } from '$lib/i18n.svelte';

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
            emailError = t('login.enterEmail');
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

<h2>{t('login.title')}</h2>
<p>{t('login.intro')}</p>

{#if submitError}
    <Alert type="error" heading={t('login.sendError')}>{submitError}</Alert>
{/if}

{#if sent}
    <Alert type="success" heading={t('login.checkEmail')}>
        {t('login.sentPrefix')} <strong>{email}</strong> {t('login.sentBody')}
    </Alert>
    {#if magicLink}
        <!-- Dev convenience: the API exposes the link directly so you can
             click it without an email server. Never shown in production. -->
        <p class="dev-magic-link">
            {t('login.devShortcut')}
            <a href={magicLink} data-testid="magic-link">{t('login.openLink')}</a>
        </p>
    {/if}
{:else}
    <Form label={t('login.formLabel')} onsubmit={handleSubmit}>
        <Field label={t('login.emailLabel')} required error={emailError}>
            <input type="email" bind:value={email} required autocomplete="email" />
        </Field>
        <div class="actions">
            <Button type="submit">{t('login.submit')}</Button>
        </div>
    </Form>
{/if}
