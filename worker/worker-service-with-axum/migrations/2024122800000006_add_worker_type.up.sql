-- Add worker_type column to workers table
-- Categorizes workers as medical providers: doctor, nurse, carer, staff, employee, manager, supervisor, consultant, other

ALTER TABLE workers ADD COLUMN worker_type VARCHAR(50)
    CHECK (worker_type IN ('doctor', 'nurse', 'carer', 'staff', 'employee', 'manager', 'supervisor', 'consultant', 'other'));

CREATE INDEX idx_workers_worker_type ON workers(worker_type);
