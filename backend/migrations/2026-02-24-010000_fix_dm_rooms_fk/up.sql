-- Fix: matrix_dm_rooms stores users.pid values, not profiles.id values.
-- Re-point foreign keys from profiles(id) to users(pid).
ALTER TABLE matrix_dm_rooms DROP CONSTRAINT matrix_dm_rooms_user_low_pid_fkey;
ALTER TABLE matrix_dm_rooms DROP CONSTRAINT matrix_dm_rooms_user_high_pid_fkey;
ALTER TABLE matrix_dm_rooms ADD CONSTRAINT matrix_dm_rooms_user_low_pid_fkey FOREIGN KEY (user_low_pid) REFERENCES users(pid);
ALTER TABLE matrix_dm_rooms ADD CONSTRAINT matrix_dm_rooms_user_high_pid_fkey FOREIGN KEY (user_high_pid) REFERENCES users(pid);
