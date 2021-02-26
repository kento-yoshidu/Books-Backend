const graphq = require('graphql');

const { GraphQLObjectType, GraphQLString } = graphql;

const BookType = new GraphqlObjectType({
  name: 'Book',
  fields: () => ({
    id: { type: GraphQLString},
    name: { type: GraphQLString},
    genre: { type: GraphQLString}
  })
})