export type Todo = {
    id: number
    text: string
    completed: boolean
    labels: Label[]
}

export type NewTodoPayload = {
    text: string
}

export type UpdateTodoPayload = {
    id: number
}

export type Label = {
    id: number
    name: string
}

export type NewLabelPayload = {
    name: string
}

