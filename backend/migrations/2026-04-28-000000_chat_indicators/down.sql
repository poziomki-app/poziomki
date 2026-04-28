ALTER TABLE public.conversation_members DROP COLUMN IF EXISTS muted_until;
DROP FUNCTION IF EXISTS app.record_delivery(uuid, int);
DROP TABLE IF EXISTS public.message_deliveries;
DROP TABLE IF EXISTS public.message_reads;
