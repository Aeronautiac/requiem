-- a boot brings the engine up to the clock itself, so how far it had got is no longer stored.

ALTER TABLE games DROP COLUMN last_reached;
