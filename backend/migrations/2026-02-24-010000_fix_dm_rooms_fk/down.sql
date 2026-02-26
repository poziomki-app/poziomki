ALTER TABLE matrix_dm_rooms DROP CONSTRAINT matrix_dm_rooms_user_low_pid_fkey;
ALTER TABLE matrix_dm_rooms DROP CONSTRAINT matrix_dm_rooms_user_high_pid_fkey;
ALTER TABLE matrix_dm_rooms ADD CONSTRAINT matrix_dm_rooms_user_low_pid_fkey FOREIGN KEY (user_low_pid) REFERENCES profiles(id);
ALTER TABLE matrix_dm_rooms ADD CONSTRAINT matrix_dm_rooms_user_high_pid_fkey FOREIGN KEY (user_high_pid) REFERENCES profiles(id);
