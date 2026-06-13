## 17. Renormalisation

```text
weighted_sum = sum(score * weight for (Some(score), weight) in components)
weight_sum   = sum(weight        for (Some(_),     weight) in components)
final        = weight_sum > 0 ? weighted_sum / weight_sum : 0
```

Two records with **identical** name + provider_id + course_code
score 1.0 because the denominator is the *present* weight, not 1.00.

