ALTER TABLE tags
ADD COLUMN parent_id UUID REFERENCES tags(id);

CREATE INDEX idx_tags_parent_id ON tags (parent_id);

CREATE TABLE event_interactions (
    profile_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    kind VARCHAR NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (profile_id, event_id, kind),
    CONSTRAINT event_interactions_kind_check CHECK (kind IN ('saved', 'joined'))
);

CREATE INDEX idx_event_interactions_profile_id ON event_interactions (profile_id);
CREATE INDEX idx_event_interactions_event_id ON event_interactions (event_id);

INSERT INTO event_interactions (profile_id, event_id, kind)
SELECT ea.profile_id, ea.event_id, 'joined'
FROM event_attendees ea
WHERE ea.status = 'going'
ON CONFLICT (profile_id, event_id, kind) DO NOTHING;

INSERT INTO tags (id, name, scope, category, parent_id) VALUES
  ('b665ff1d-52e3-4efc-9b68-1f53d2efad10', 'Sport', 'interest', 'root', NULL),
  ('a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d', 'Muzyka', 'interest', 'root', NULL),
  ('348f3d0c-7bf4-4c7d-8b5c-31b254ec9f34', 'Sztuka', 'interest', 'root', NULL),
  ('11c8d9bc-95ea-4a79-8f3c-c79d6dfac6e8', 'Film i scena', 'interest', 'root', NULL),
  ('20f0febb-cfc4-4b5a-a4a4-140ff8af9abc', 'Technologia', 'interest', 'root', NULL),
  ('7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6', 'Nauka i edukacja', 'interest', 'root', NULL),
  ('3301fcb7-f2d0-4f3e-bf17-9fb8af4a1c84', 'Podróże i przygody', 'interest', 'root', NULL),
  ('77d4f8b0-c030-4ed1-8d75-697c15a69f05', 'Kulinaria', 'interest', 'root', NULL),
  ('566e1714-0ec4-4d52-8562-fca84e2c8419', 'Literatura', 'interest', 'root', NULL),
  ('a89488ea-43f1-4c72-94dd-fc3747fb95a0', 'Gry', 'interest', 'root', NULL),
  ('63318021-e21d-4d7d-a4cb-f5e0f15fc833', 'Społeczność', 'interest', 'root', NULL),
  ('460c6106-6f65-4f0d-bbf8-ef49687ec0f3', 'Styl życia', 'interest', 'root', NULL)
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  category = EXCLUDED.category,
  parent_id = EXCLUDED.parent_id,
  updated_at = NOW();

UPDATE tags
SET parent_id = CASE category
  WHEN 'sport' THEN 'b665ff1d-52e3-4efc-9b68-1f53d2efad10'::uuid
  WHEN 'muzyka' THEN 'a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d'::uuid
  WHEN 'sztuka' THEN '348f3d0c-7bf4-4c7d-8b5c-31b254ec9f34'::uuid
  WHEN 'film' THEN '11c8d9bc-95ea-4a79-8f3c-c79d6dfac6e8'::uuid
  WHEN 'technologia' THEN '20f0febb-cfc4-4b5a-a4a4-140ff8af9abc'::uuid
  WHEN 'nauka' THEN '7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6'::uuid
  WHEN 'podroze' THEN '3301fcb7-f2d0-4f3e-bf17-9fb8af4a1c84'::uuid
  WHEN 'kulinaria' THEN '77d4f8b0-c030-4ed1-8d75-697c15a69f05'::uuid
  WHEN 'literatura' THEN '566e1714-0ec4-4d52-8562-fca84e2c8419'::uuid
  WHEN 'gry' THEN 'a89488ea-43f1-4c72-94dd-fc3747fb95a0'::uuid
  WHEN 'spolecznosc' THEN '63318021-e21d-4d7d-a4cb-f5e0f15fc833'::uuid
  WHEN 'styl_zycia' THEN '460c6106-6f65-4f0d-bbf8-ef49687ec0f3'::uuid
  ELSE parent_id
END
WHERE scope = 'interest';

INSERT INTO tags (id, name, scope, category, parent_id) VALUES
  ('f77d23b0-28bf-4db0-82bf-1efd66f8244e', 'Bieg grupowy', 'event', 'sport', 'a84a5bec-3203-5add-8def-e90f67c7c981'),
  ('f8398c10-ea1a-4b8d-a33f-80c91bb4270f', 'Sesja jogi', 'event', 'sport', 'b9975429-50e3-5b82-a3ff-5288a153043c'),
  ('00d91f4a-f275-4744-b08b-f396f0e841b1', 'Mecz amatorski', 'event', 'sport', 'b665ff1d-52e3-4efc-9b68-1f53d2efad10'),
  ('f057fe98-62c5-4c54-bfea-afc2cc74e07d', 'Jam session', 'event', 'muzyka', 'a4cc2ff7-122a-4473-9b8d-3ddf4d61db8d'),
  ('2f61bc8f-54d7-4962-b468-4b3abf9b7626', 'Koncert na żywo', 'event', 'muzyka', '76dc48f2-cd4f-57f8-9642-f54381e1c5d6'),
  ('153b6c59-e6de-453f-a6df-f52132d1f77c', 'Wystawa', 'event', 'sztuka', '348f3d0c-7bf4-4c7d-8b5c-31b254ec9f34'),
  ('636bc541-8282-4255-b0b8-a3a72e738932', 'Warsztaty kreatywne', 'event', 'sztuka', '348f3d0c-7bf4-4c7d-8b5c-31b254ec9f34'),
  ('8ae53725-2867-48ac-925c-e97cfed87aa7', 'Wieczór filmowy', 'event', 'film', '47db0957-99f4-5853-a56d-372a652c3f6c'),
  ('67de9126-c32b-43f1-a84a-88142cd0eac9', 'Stand-up na żywo', 'event', 'film', 'e573e23d-a9ae-5152-b397-0ba84c6789d0'),
  ('6e605dd1-411a-4b42-b616-64221c1c9768', 'Hackathon', 'event', 'technologia', 'db581004-0cd8-55f7-9f99-d608b2eb312e'),
  ('e574bb25-9808-4d40-9607-200f70c72ad9', 'Meetup technologiczny', 'event', 'technologia', '20f0febb-cfc4-4b5a-a4a4-140ff8af9abc'),
  ('f14ac9dc-8623-4971-8f0f-5b82e80a2e8f', 'Koło naukowe', 'event', 'nauka', '7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6'),
  ('af14d14c-fe6e-48c6-a76c-9c2226051381', 'Grupa nauki', 'event', 'nauka', '7892a8b2-dc1d-4082-a6b3-23fe9a3f51f6'),
  ('f2bc5778-aeb2-4343-a2b6-d1e818c15b31', 'Spacer miejski', 'event', 'podroze', 'ea0eed42-a581-531c-abab-b7658a6a774e'),
  ('1cafdf26-13de-4bdb-acb8-bb2682dd2bf8', 'Wyjazd outdoorowy', 'event', 'podroze', '3301fcb7-f2d0-4f3e-bf17-9fb8af4a1c84'),
  ('df9cd37d-3055-4d17-ac32-0c6b860df06f', 'Degustacja', 'event', 'kulinaria', '77d4f8b0-c030-4ed1-8d75-697c15a69f05'),
  ('6d2fb7b3-1c72-4c13-98bd-f5f48b53c7cb', 'Warsztaty kulinarne', 'event', 'kulinaria', 'f9399a56-da53-5c96-8d04-2ee25edc57d3'),
  ('f1cc04b8-1c1d-4556-b6fb-8e7b4ac5e376', 'Klub książki', 'event', 'literatura', '9b7b02f9-0a4a-55ba-a96b-f7cf95df4301'),
  ('5b75dc99-181d-404a-a5dc-3b15c7e25db5', 'Pisanie razem', 'event', 'literatura', 'a68fd876-3921-531d-abb9-a534c9ae3b07'),
  ('d01991c9-00f2-49ab-94f2-c568ec336f42', 'Wieczór planszówek', 'event', 'gry', '5bd831a9-724a-5e0b-ab5e-cde71d40aa58'),
  ('4217f53b-ec49-423d-9b96-3d41d0e31d64', 'Sesja RPG', 'event', 'gry', '6a64142c-2196-5c9f-8cd8-ab29fd39d400'),
  ('a2cdb278-fb19-48b8-8438-8110afcbdf1f', 'Wolontariat', 'event', 'spolecznosc', 'edc67245-3596-538a-aa57-d2526cfceb57'),
  ('4586b307-c167-456d-abd2-0d3ef65afdc4', 'Debata', 'event', 'spolecznosc', '0b969f83-bf34-5522-9f01-05fdd7579bb2'),
  ('182039cc-3ae5-4953-a2a1-b9b1135cd1c1', 'Krąg medytacji', 'event', 'styl_zycia', 'b576c7ff-2038-5530-80e5-e313ebfd475c'),
  ('b9c896e9-5d49-4847-8c58-9ed77efc5f0d', 'Warsztaty tańca', 'event', 'styl_zycia', 'f4b20bdc-a473-5b6c-a38c-d7ca291ffa38')
ON CONFLICT (id) DO UPDATE SET
  name = EXCLUDED.name,
  category = EXCLUDED.category,
  parent_id = EXCLUDED.parent_id,
  updated_at = NOW();
