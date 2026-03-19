CREATE OR REPLACE FUNCTION sync_monthly_token_totals()
RETURNS TRIGGER AS $$
DECLARE
    ym INT;
    billable BIGINT;
BEGIN
    IF TG_OP = 'INSERT' THEN
        ym := EXTRACT(YEAR FROM NEW.created_at)::INT * 100
            + EXTRACT(MONTH FROM NEW.created_at)::INT;
        billable := NEW.input_tokens + NEW.output_tokens + NEW.reasoning_tokens;
        INSERT INTO monthly_token_totals (user_id, year_month, total_tokens)
        VALUES (NEW.user_id, ym, billable)
        ON CONFLICT (user_id, year_month)
        DO UPDATE SET total_tokens = monthly_token_totals.total_tokens + EXCLUDED.total_tokens;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;
