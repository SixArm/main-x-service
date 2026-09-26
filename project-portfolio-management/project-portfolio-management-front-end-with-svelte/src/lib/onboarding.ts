// Operator onboarding guide (T-28p): a role-by-role "first hour"
// walkthrough, each step naming the route it lands on. Pure data —
// no API calls, no claim about how long any of it actually takes.
//
// This makes **no time-to-productivity claim**: nobody has measured
// how long a first hour with this guide actually takes, and a guide
// that invented a number would be exactly the kind of confident-
// looking figure this family's other views refuse to produce from
// nothing. See `spec/index.md` §13 T-28p.

export type OnboardingStep = {
  /** The route this step lands on — a real page in this app. */
  route: string;
  /** What the operator does or looks at on this page. */
  label: string;
  /** Why this step matters for this role. */
  note: string;
};

export type OnboardingRole = {
  id: string;
  title: string;
  summary: string;
  steps: OnboardingStep[];
};

export const ONBOARDING_ROLES: OnboardingRole[] = [
  {
    id: "executive",
    title: "Executive",
    summary: "Outcomes and exposure across the whole estate, not any one plan.",
    steps: [
      {
        route: "/executive",
        label: "Read the executive summary",
        note: "Health, decisions, benefits realized, and strategic alignment in one view.",
      },
      {
        route: "/dashboard",
        label: "Open the at-a-glance dashboard",
        note: "The whole estate's RAG status and schedule violations at once.",
      },
      {
        route: "/board",
        label: "Review the board pack",
        note: "The pack a board meeting would actually be handed: investments and trends.",
      },
      {
        route: "/financials",
        label: "Check budget variance and exposure",
        note: "Where spend is diverging from plan, per currency — currencies are never summed together.",
      },
      {
        route: "/scenarios",
        label: "Compare two candidate portfolios",
        note: "What-if scenarios side by side, before committing funding to either.",
      },
      {
        route: "/prioritisation",
        label: "See what's ranked highest, and why",
        note: "The Smart Score queue — every score shows the evidence behind it, or says it has none.",
      },
    ],
  },
  {
    id: "pmo",
    title: "PMO / portfolio manager",
    summary: "Intake, governance, and the schedule across every plan.",
    steps: [
      {
        route: "/proposals",
        label: "Open the intake pipeline",
        note: "Every proposal's stage, from draft through approval.",
      },
      {
        route: "/plans/new",
        label: "Register a plan",
        note: "The first plan you create is a good way to see the whole payload shape.",
      },
      {
        route: "/plans",
        label: "Browse the plan registry",
        note: "One recursive collection — `kind` is a label, not a gate.",
      },
      {
        route: "/plans/[pid]/governance",
        label: "Open a plan's governance tab",
        note: "Gate reviews, risks, and budget lines for that one plan.",
      },
      {
        route: "/gantt",
        label: "Look at the cross-plan schedule",
        note: "Dependencies, the critical path, and any slipping successor.",
      },
      {
        route: "/risk",
        label: "Check the risk register",
        note: "Everything open or mitigating, ranked by exposure.",
      },
      {
        route: "/compliance",
        label: "Review the compliance register",
        note: "Standing findings across the estate, not just one plan's.",
      },
      {
        route: "/reports",
        label: "Run a saved report",
        note: "A filter + field projection saved for reuse, run on demand.",
      },
    ],
  },
  {
    id: "resource-manager",
    title: "Resource manager",
    summary: "Who is allocated where, and how delivery is actually flowing.",
    steps: [
      {
        route: "/capacity",
        label: "Check the capacity rollup",
        note: "Who is over-allocated this window, summed across every plan they're on.",
      },
      {
        route: "/plans/[pid]/board",
        label: "Open a plan's task board",
        note: "The Kanban board that task-level work actually moves through.",
      },
      {
        route: "/plans/[pid]/flow",
        label: "Read that plan's flow figures",
        note: "Cycle time, lead time, and flow efficiency — derived from the board's own history.",
      },
      {
        route: "/engineering",
        label: "Scan blocked items and the delivery mix",
        note: "What's stuck, the MoSCoW mix, and the milestone calendar in one view.",
      },
      {
        route: "/calendar",
        label: "Look at the milestone calendar",
        note: "Every milestone across every plan, due dates included.",
      },
      {
        route: "/automations",
        label: "See what automations are configured",
        note: "What fires on a field change, and what it does when it does.",
      },
      {
        route: "/reviews",
        label: "Open collaborative reviews",
        note: "Where a plan gets a second opinion before a decision is made on it.",
      },
    ],
  },
];
