import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { useState } from "react";
import { RepositoryBrowser } from "./RepositoryBrowser";
import type { Repository } from "../../types";

const CATALOG_KEY = "rm-mock-catalog";

function StatefulBrowser({
  initialRepositories,
  onRepositoryDelete,
}: {
  initialRepositories: Repository[];
  onRepositoryDelete: (repository: string) => Promise<void>;
}) {
  const [repositories, setRepositories] = useState(initialRepositories);

  return (
    <RepositoryBrowser
      repositories={repositories}
      search=""
      profileId="p1"
      registryUrl="http://localhost:5000"
      onSearchChange={() => {}}
      onRepositorySelect={() => {}}
      onRepositoryDelete={async (repository) => {
        await onRepositoryDelete(repository);
        setRepositories((current) => current.filter((repo) => repo.name !== repository));
      }}
    />
  );
}

describe("RepositoryBrowser repository delete", () => {
  afterEach(() => {
    localStorage.removeItem(CATALOG_KEY);
  });

  it("shows repository delete impact before confirming", async () => {
    localStorage.setItem(CATALOG_KEY, "true");
    const user = userEvent.setup();

    render(
      <RepositoryBrowser
        repositories={[{ name: "alpine", tagCount: 1 }]}
        search=""
        profileId="p1"
        registryUrl="http://localhost:5000"
        onSearchChange={() => {}}
        onRepositorySelect={() => {}}
        onRepositoryDelete={async () => {}}
      />,
    );

    await user.click(screen.getByTestId("rm-repository-delete-button"));

    expect(await screen.findByText(/confirm repository delete/i)).toBeVisible();
    expect(screen.getByTestId("delete-impact-list")).toBeVisible();
    expect(await screen.findByText(/total tags/i)).toBeVisible();
    expect(await screen.findByText(/unique digests/i)).toBeVisible();
    expect(await screen.findByText(/affected tags/i)).toBeVisible();
  });

  it("removes the repository card after a successful delete reduces tag count to zero", async () => {
    localStorage.setItem(CATALOG_KEY, "true");
    const user = userEvent.setup();
    const onDelete = vi.fn(async () => {});

    render(
      <StatefulBrowser
        initialRepositories={[{ name: "alpine", tagCount: 1 }]}
        onRepositoryDelete={onDelete}
      />,
    );

    expect(screen.getByTestId("rm-repository-card")).toBeVisible();

    await user.click(screen.getByTestId("rm-repository-delete-button"));
    await screen.findByText(/confirm repository delete/i);

    await user.click(screen.getByRole("button", { name: /delete repository/i }));

    await waitFor(() => {
      expect(screen.queryByTestId("rm-repository-card")).not.toBeInTheDocument();
    });
    expect(onDelete).toHaveBeenCalledWith("alpine");
  });
});
