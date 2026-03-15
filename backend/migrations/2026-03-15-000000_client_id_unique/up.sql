CREATE UNIQUE INDEX idx_messages_client_id
    ON messages (conversation_id, sender_id, client_id)
    WHERE client_id IS NOT NULL;
