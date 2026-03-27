import { createContext, useContext, type ReactNode } from "react";
import type { LegionService } from "./interface";

const LegionContext = createContext<LegionService | null>(null);

interface LegionProviderProps {
  adapter: LegionService;
  children: ReactNode;
}

export function LegionProvider({ adapter, children }: LegionProviderProps) {
  return (
    <LegionContext.Provider value={adapter}>{children}</LegionContext.Provider>
  );
}

export function useLegion(): LegionService {
  const service = useContext(LegionContext);
  if (!service) {
    throw new Error("useLegion must be used within a LegionProvider");
  }
  return service;
}
