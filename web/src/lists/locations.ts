// The projects and sections a task can be moved to (the Move to… list): given by
// the page, since a row only knows where its own task is.
import { createContext } from "react";
import type { Project, Section } from "../bindings";

export type Locations = { projects: Project[]; sections: Section[] };

export const LocationsContext = createContext<Locations>({ projects: [], sections: [] });
