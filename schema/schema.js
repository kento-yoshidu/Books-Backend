const graphql = require('graphql');
const _ = require('lodash');

const { GraphQLObjectType, GraphQLString, GraphQLSchema } = graphql;

// dummy data

const book = [
  { name: "Kento", genre: "Fantasy", id: "1"},
  { name: "hikari", genre: "Fantasy", id: "2"},
  { name: "Kento", genre: "Sci", id: "3"},
];

const BookType = new GraphQLObjectType({
  name: 'Book',
  fields: () => ({
    id: { type: GraphQLString},
    name: { type: GraphQLString},
    genre: { type: GraphQLString}
  })
});

const RootQuery = new GraphQLObjectType({
  name: 'RootQueryTypes',
  fields: {
    book: {
      type: BookType,
      args: { id: { type: GraphQLString } },
      resolve(parent, args) {
        return _.find(book, { id: args.id})
      }
    }
  }
});

module.exports = new GraphQLSchema({
  query: RootQuery
});