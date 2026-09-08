import { useEffect, useState } from "react";
import { Overlay, type Frame } from "tauri-overlay";

import { useLockInPets } from "../lockin/useLockInPets.ts";
import { loadPetManifest, type PetPlacement } from "./petManifest.ts";

// bage-rate's overlay: the pets from the manifest, plus whatever lock-in mode spawns.
export function PetsOverlay() {
  const [pets, setPets] = useState<PetPlacement[]>([]);
  const [error, setError] = useState<string | null>(null);
  const lockIn = useLockInPets();

  useEffect(() => {
    loadPetManifest().then(setPets, (reason: unknown) => setError(String(reason)));
  }, []);

  if (error) return <p className="error">{error}</p>;
  const frames: Frame[] = [
    ...pets.map(({ file, ...box }, index): Frame => ({ key: `pet-${index}`, src: `/pets/${file}`, ...box })),
    ...lockIn.pets,
  ];
  return (
    <Overlay frames={frames}>
      {lockIn.pets.length > 0 && (
        // ponytail: catches the whole screen, not just the drawn pixels; hit-test frame pixels if that matters.
        <div className="lockin-catch clickable" title="Back to work" onClick={lockIn.refocus} />
      )}
      {lockIn.error && <div className="lockin-badge" role="alert">{lockIn.error}</div>}
      {!lockIn.error && lockIn.anchor && <div className="lockin-badge">🔒 Locked in: {lockIn.anchor}</div>}
    </Overlay>
  );
}
