-- Remove event_tags/profile_tags referencing the new event-scope tags
DELETE FROM profile_tags
WHERE tag_id IN (
  'f77d23b0-28bf-4db0-82bf-1efd66f8244e',
  'f8398c10-ea1a-4b8d-a33f-80c91bb4270f',
  '00d91f4a-f275-4744-b08b-f396f0e841b1',
  'f057fe98-62c5-4c54-bfea-afc2cc74e07d',
  '2f61bc8f-54d7-4962-b468-4b3abf9b7626',
  '153b6c59-e6de-453f-a6df-f52132d1f77c',
  '636bc541-8282-4255-b0b8-a3a72e738932',
  '8ae53725-2867-48ac-925c-e97cfed87aa7',
  '67de9126-c32b-43f1-a84a-88142cd0eac9',
  '6e605dd1-411a-4b42-b616-64221c1c9768',
  'e574bb25-9808-4d40-9607-200f70c72ad9',
  'f14ac9dc-8623-4971-8f0f-5b82e80a2e8f',
  'af14d14c-fe6e-48c6-a76c-9c2226051381',
  'f2bc5778-aeb2-4343-a2b6-d1e818c15b31',
  '1cafdf26-13de-4bdb-acb8-bb2682dd2bf8',
  'df9cd37d-3055-4d17-ac32-0c6b860df06f',
  '6d2fb7b3-1c72-4c13-98bd-f5f48b53c7cb',
  'f1cc04b8-1c1d-4556-b6fb-8e7b4ac5e376',
  '5b75dc99-181d-404a-a5dc-3b15c7e25db5',
  'd01991c9-00f2-49ab-94f2-c568ec336f42',
  '4217f53b-ec49-423d-9b96-3d41d0e31d64',
  'a2cdb278-fb19-48b8-8438-8110afcbdf1f',
  '4586b307-c167-456d-abd2-0d3ef65afdc4',
  '182039cc-3ae5-4953-a2a1-b9b1135cd1c1',
  'b9c896e9-5d49-4847-8c58-9ed77efc5f0d'
);

DELETE FROM event_tags
WHERE tag_id IN (
  'f77d23b0-28bf-4db0-82bf-1efd66f8244e',
  'f8398c10-ea1a-4b8d-a33f-80c91bb4270f',
  '00d91f4a-f275-4744-b08b-f396f0e841b1',
  'f057fe98-62c5-4c54-bfea-afc2cc74e07d',
  '2f61bc8f-54d7-4962-b468-4b3abf9b7626',
  '153b6c59-e6de-453f-a6df-f52132d1f77c',
  '636bc541-8282-4255-b0b8-a3a72e738932',
  '8ae53725-2867-48ac-925c-e97cfed87aa7',
  '67de9126-c32b-43f1-a84a-88142cd0eac9',
  '6e605dd1-411a-4b42-b616-64221c1c9768',
  'e574bb25-9808-4d40-9607-200f70c72ad9',
  'f14ac9dc-8623-4971-8f0f-5b82e80a2e8f',
  'af14d14c-fe6e-48c6-a76c-9c2226051381',
  'f2bc5778-aeb2-4343-a2b6-d1e818c15b31',
  '1cafdf26-13de-4bdb-acb8-bb2682dd2bf8',
  'df9cd37d-3055-4d17-ac32-0c6b860df06f',
  '6d2fb7b3-1c72-4c13-98bd-f5f48b53c7cb',
  'f1cc04b8-1c1d-4556-b6fb-8e7b4ac5e376',
  '5b75dc99-181d-404a-a5dc-3b15c7e25db5',
  'd01991c9-00f2-49ab-94f2-c568ec336f42',
  '4217f53b-ec49-423d-9b96-3d41d0e31d64',
  'a2cdb278-fb19-48b8-8438-8110afcbdf1f',
  '4586b307-c167-456d-abd2-0d3ef65afdc4',
  '182039cc-3ae5-4953-a2a1-b9b1135cd1c1',
  'b9c896e9-5d49-4847-8c58-9ed77efc5f0d'
);

DELETE FROM tags
WHERE id IN (
  'f77d23b0-28bf-4db0-82bf-1efd66f8244e',
  'f8398c10-ea1a-4b8d-a33f-80c91bb4270f',
  '00d91f4a-f275-4744-b08b-f396f0e841b1',
  'f057fe98-62c5-4c54-bfea-afc2cc74e07d',
  '2f61bc8f-54d7-4962-b468-4b3abf9b7626',
  '153b6c59-e6de-453f-a6df-f52132d1f77c',
  '636bc541-8282-4255-b0b8-a3a72e738932',
  '8ae53725-2867-48ac-925c-e97cfed87aa7',
  '67de9126-c32b-43f1-a84a-88142cd0eac9',
  '6e605dd1-411a-4b42-b616-64221c1c9768',
  'e574bb25-9808-4d40-9607-200f70c72ad9',
  'f14ac9dc-8623-4971-8f0f-5b82e80a2e8f',
  'af14d14c-fe6e-48c6-a76c-9c2226051381',
  'f2bc5778-aeb2-4343-a2b6-d1e818c15b31',
  '1cafdf26-13de-4bdb-acb8-bb2682dd2bf8',
  'df9cd37d-3055-4d17-ac32-0c6b860df06f',
  '6d2fb7b3-1c72-4c13-98bd-f5f48b53c7cb',
  'f1cc04b8-1c1d-4556-b6fb-8e7b4ac5e376',
  '5b75dc99-181d-404a-a5dc-3b15c7e25db5',
  'd01991c9-00f2-49ab-94f2-c568ec336f42',
  '4217f53b-ec49-423d-9b96-3d41d0e31d64',
  'a2cdb278-fb19-48b8-8438-8110afcbdf1f',
  '4586b307-c167-456d-abd2-0d3ef65afdc4',
  '182039cc-3ae5-4953-a2a1-b9b1135cd1c1',
  'b9c896e9-5d49-4847-8c58-9ed77efc5f0d'
);

-- Clear parent_id on interest tags pointing to root categories
UPDATE tags
SET parent_id = NULL
WHERE parent_id IN (
  'b665ff1d-52e3-4efc-9b68-1f53d2efad10',
  'a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d',
  '348f3d0c-7bf4-4c7d-8b5c-31b254ec9f34',
  '11c8d9bc-95ea-4a79-8f3c-c79d6dfac6e8',
  '20f0febb-cfc4-4b5a-a4a4-140ff8af9abc',
  '7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6',
  '3301fcb7-f2d0-4f3e-bf17-9fb8af4a1c84',
  '77d4f8b0-c030-4ed1-8d75-697c15a69f05',
  '566e1714-0ec4-4d52-8562-fca84e2c8419',
  'a89488ea-43f1-4c72-94dd-fc3747fb95a0',
  '63318021-e21d-4d7d-a4cb-f5e0f15fc833',
  '460c6106-6f65-4f0d-bbf8-ef49687ec0f3'
);

-- Remove event_tags/profile_tags referencing root category tags
DELETE FROM event_tags
WHERE tag_id IN (
  'b665ff1d-52e3-4efc-9b68-1f53d2efad10',
  'a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d',
  '348f3d0c-7bf4-4c7d-8b5c-31b254ec9f34',
  '11c8d9bc-95ea-4a79-8f3c-c79d6dfac6e8',
  '20f0febb-cfc4-4b5a-a4a4-140ff8af9abc',
  '7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6',
  '3301fcb7-f2d0-4f3e-bf17-9fb8af4a1c84',
  '77d4f8b0-c030-4ed1-8d75-697c15a69f05',
  '566e1714-0ec4-4d52-8562-fca84e2c8419',
  'a89488ea-43f1-4c72-94dd-fc3747fb95a0',
  '63318021-e21d-4d7d-a4cb-f5e0f15fc833',
  '460c6106-6f65-4f0d-bbf8-ef49687ec0f3'
);

DELETE FROM profile_tags
WHERE tag_id IN (
  'b665ff1d-52e3-4efc-9b68-1f53d2efad10',
  'a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d',
  '348f3d0c-7bf4-4c7d-8b5c-31b254ec9f34',
  '11c8d9bc-95ea-4a79-8f3c-c79d6dfac6e8',
  '20f0febb-cfc4-4b5a-a4a4-140ff8af9abc',
  '7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6',
  '3301fcb7-f2d0-4f3e-bf17-9fb8af4a1c84',
  '77d4f8b0-c030-4ed1-8d75-697c15a69f05',
  '566e1714-0ec4-4d52-8562-fca84e2c8419',
  'a89488ea-43f1-4c72-94dd-fc3747fb95a0',
  '63318021-e21d-4d7d-a4cb-f5e0f15fc833',
  '460c6106-6f65-4f0d-bbf8-ef49687ec0f3'
);

DELETE FROM tags
WHERE id IN (
  'b665ff1d-52e3-4efc-9b68-1f53d2efad10',
  'a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d',
  '348f3d0c-7bf4-4c7d-8b5c-31b254ec9f34',
  '11c8d9bc-95ea-4a79-8f3c-c79d6dfac6e8',
  '20f0febb-cfc4-4b5a-a4a4-140ff8af9abc',
  '7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6',
  '3301fcb7-f2d0-4f3e-bf17-9fb8af4a1c84',
  '77d4f8b0-c030-4ed1-8d75-697c15a69f05',
  '566e1714-0ec4-4d52-8562-fca84e2c8419',
  'a89488ea-43f1-4c72-94dd-fc3747fb95a0',
  '63318021-e21d-4d7d-a4cb-f5e0f15fc833',
  '460c6106-6f65-4f0d-bbf8-ef49687ec0f3'
);

DROP TRIGGER IF EXISTS set_event_interactions_updated_at ON event_interactions;
DROP TABLE IF EXISTS event_interactions;
DROP FUNCTION IF EXISTS set_updated_at();

DROP INDEX IF EXISTS idx_tags_parent_id;
ALTER TABLE tags DROP COLUMN IF EXISTS parent_id;
