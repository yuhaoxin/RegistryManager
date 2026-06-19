import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { RepositoryCard } from "./RepositoryCard";

const mockRepository = { name: "alpine", tagCount: 2, size: "12 MB", lastUpdated: "1 hour ago" };

describe("RepositoryCard", () => {
  it("shows the delete button for a local registry", () => {
    render(
      <RepositoryCard
        repository={mockRepository}
        registryUrl="http://localhost:5000"
        profileId="p1"
        onClick={() => {}}
      />,
    );

    expect(screen.getByTestId("rm-repository-delete-button")).toBeVisible();
    expect(screen.queryByTestId("rm-repository-delete-disabled")).not.toBeInTheDocument();
  });

  it("shows disabled explanation for a remote registry", () => {
    render(
      <RepositoryCard
        repository={mockRepository}
        registryUrl="https://registry.example.com"
        profileId="p1"
        onClick={() => {}}
      />,
    );

    expect(screen.queryByTestId("rm-repository-delete-button")).not.toBeInTheDocument();
    expect(screen.getByTestId("rm-repository-delete-disabled")).toHaveTextContent(/remote delete disabled/i);
  });

  it("hides the delete button when there is no profile id", () => {
    render(
      <RepositoryCard
        repository={mockRepository}
        registryUrl="http://localhost:5000"
        onClick={() => {}}
      />,
    );

    expect(screen.queryByTestId("rm-repository-delete-button")).not.toBeInTheDocument();
    expect(screen.getByTestId("rm-repository-delete-disabled")).toHaveTextContent(/no tags to delete/i);
  });

  it("requests delete when the delete button is clicked", async () => {
    const user = userEvent.setup();
    const onDeleteRequest = vi.fn();
    const onClick = vi.fn();

    render(
      <RepositoryCard
        repository={mockRepository}
        registryUrl="http://localhost:5000"
        profileId="p1"
        onClick={onClick}
        onDeleteRequest={onDeleteRequest}
      />,
    );

    await user.click(screen.getByTestId("rm-repository-delete-button"));

    expect(onDeleteRequest).toHaveBeenCalledWith(mockRepository);
    expect(onClick).not.toHaveBeenCalled();
  });
});
