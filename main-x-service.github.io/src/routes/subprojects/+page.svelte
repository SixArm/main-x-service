<script lang="ts">
    const REPO = "https://github.com/SixArm/main-x-service/tree/main";

    type Row = { name: string; path: string; note: string };

    const services: Row[] = [
        { name: "person-service", path: "person/person-service-with-loco", note: "Person (general)" },
        { name: "worker-service", path: "worker/worker-service-with-loco", note: "Worker (workforce / professional, incl. assessments)" },
        { name: "place-service", path: "place/place-service-with-loco", note: "Place (schema.org/Place)" },
        { name: "thing-service", path: "thing/thing-service-with-loco", note: "Thing (schema.org/Thing)" },
        { name: "event-service", path: "event/event-service-with-loco", note: "Event (schema.org/Event, time-bounded)" },
        { name: "course-service", path: "course/course-service-with-loco", note: "Course (+ CourseInstance sub-resource)" },
        { name: "authentication-service", path: "authentication/authentication-service-with-loco", note: "User — the central SSO provider" },
        { name: "organization-service", path: "organization/organization-service-with-loco", note: "Organization (schema.org/Organization)" },
        { name: "care-pathway-service", path: "care-pathway/care-pathway-service-with-loco", note: "Care pathway (clinical pathway)" },
        { name: "case-service", path: "case/case-service-with-loco", note: "Case (governmental case tracking)" },
        { name: "project-portfolio-management-service", path: "project-portfolio-management/project-portfolio-management-service-with-loco", note: "Plan (one recursive tree; kind is a label, not a gate)" },
    ];

    const matchers: Row[] = [
        { name: "person-matcher", path: "person/person-matcher-rust-crate", note: "Person" },
        { name: "worker-matcher", path: "worker/worker-matcher-rust-crate", note: "Worker" },
        { name: "place-matcher", path: "place/place-matcher-rust-crate", note: "Place" },
        { name: "thing-matcher", path: "thing/thing-matcher-rust-crate", note: "Thing" },
        { name: "event-matcher", path: "event/event-matcher-rust-crate", note: "Event" },
        { name: "course-matcher", path: "course/course-matcher-rust-crate", note: "Course" },
        { name: "organization-matcher", path: "organization/organization-matcher-rust-crate", note: "Organization" },
        { name: "care-pathway-matcher", path: "care-pathway/care-pathway-matcher-rust-crate", note: "Care pathway" },
        { name: "case-matcher", path: "case/case-matcher-rust-crate", note: "Case" },
        { name: "project-portfolio-management-matcher", path: "project-portfolio-management/project-portfolio-management-matcher-rust-crate", note: "Plan" },
    ];

    const libraries: Row[] = [
        { name: "authentication-verifier", path: "authentication/authentication-verifier-rust-crate", note: "Peer-side offline PASETO v4.public verification. Published to crates.io." },
        { name: "integrity-mac", path: "integrity/integrity-mac-rust-crate", note: "Keyed integrity MACs (HMAC-SHA256) with production-grade key handling. In-tree only." },
        { name: "entity-ref", path: "link/entity-ref-rust-crate", note: "The EntityRef URN + closed EdgeKind registry. Published to crates.io." },
    ];

    const frontEnds: Row[] = [
        { name: "person-front-end-with-svelte", path: "person/person-front-end-with-svelte", note: "person-service" },
        { name: "worker-front-end-with-svelte", path: "worker/worker-front-end-with-svelte", note: "worker-service" },
        { name: "place-front-end-with-svelte", path: "place/place-front-end-with-svelte", note: "place-service" },
        { name: "thing-front-end-with-svelte", path: "thing/thing-front-end-with-svelte", note: "thing-service" },
        { name: "event-front-end-with-svelte", path: "event/event-front-end-with-svelte", note: "event-service" },
        { name: "course-front-end-with-svelte", path: "course/course-front-end-with-svelte", note: "course-service" },
        { name: "authentication-front-end-with-svelte", path: "authentication/authentication-front-end-with-svelte", note: "authentication-service" },
        { name: "organization-front-end-with-svelte", path: "organization/organization-front-end-with-svelte", note: "organization-service" },
        { name: "care-pathway-front-end-with-svelte", path: "care-pathway/care-pathway-front-end-with-svelte", note: "care-pathway-service" },
        { name: "case-front-end-with-svelte", path: "case/case-front-end-with-svelte", note: "case-service" },
        { name: "project-portfolio-management-front-end-with-svelte", path: "project-portfolio-management/project-portfolio-management-front-end-with-svelte", note: "project-portfolio-management-service" },
    ];

    const consumerApps: Row[] = [
        { name: "case-folder", path: "case-folder", note: "NHS paper case-note folder location tracking" },
        { name: "patient-flow", path: "patient-flow", note: "NHS hospital patient flow and bed management" },
        { name: "workforce-planning-management", path: "workforce-planning-management", note: "All-in-one HR across the employee lifecycle" },
        { name: "contact-relationship-management", path: "contact-relationship-management", note: "Customer and prospect relationships" },
        { name: "content-management-system", path: "content-management-system", note: "Headless content management" },
    ];

    const crossCutting: Row[] = [
        { name: "link-graph-service-with-loco", path: "link/link-graph-service-with-loco", note: "Read-model aggregator for cross-service entity linking" },
    ];

    const sections: { title: string; rows: Row[]; nameLabel: string }[] = [
        { title: "Entity registry services (Rust, loco.rs)", rows: services, nameLabel: "Crate" },
        { title: "Matcher crates", rows: matchers, nameLabel: "Crate" },
        { title: "Library crates", rows: libraries, nameLabel: "Crate" },
        { title: "Front-end projects (SvelteKit)", rows: frontEnds, nameLabel: "Project" },
        { title: "Consumer applications", rows: consumerApps, nameLabel: "Application" },
        { title: "Cross-cutting services", rows: crossCutting, nameLabel: "Service" },
    ];
</script>

<svelte:head><title>Subprojects · Main X Index</title></svelte:head>

<h1>Subprojects</h1>

<p>
    The full family index — every crate, matcher, library, front-end,
    consumer app, and cross-cutting service in the monorepo, each linking
    to its source. See each subproject's own <code>spec/</code> for the
    authoritative detail; this page mirrors the root
    <a href="https://github.com/SixArm/main-x-service/blob/main/AGENTS.md">AGENTS.md</a>
    index.
</p>

{#each sections as section}
    <h2>{section.title}</h2>
    <table>
        <thead>
            <tr><th>{section.nameLabel}</th><th>Notes</th></tr>
        </thead>
        <tbody>
            {#each section.rows as row}
                <tr>
                    <td><a href={`${REPO}/${row.path}`}>{row.name}</a></td>
                    <td>{row.note}</td>
                </tr>
            {/each}
        </tbody>
    </table>
{/each}
